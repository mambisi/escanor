use crate::geo::{Circle, GeoPoint2D};
use crate::unit_conv::*;
use std::collections::{BTreeMap, HashSet, HashMap};
use std::sync::{Arc, Mutex, RwLockReadGuard, RwLockWriteGuard};
use std::sync::RwLock;
use rstar::RTree;
use crate::{util, file_dirs};
use crate::command::*;
use lazy_static::lazy_static;
use crate::printer::*;
use crate::error::CustomMessageError;
use geohash;
use geohash::Coordinate;
use crate::util::Location;
use serde::{Serialize, Deserialize};
use log::Level::{Info, Debug};
use glob::Pattern;

extern crate chrono;

use serde_json::Value;
use tokio::time;
use std::time::{Duration, SystemTime};
use chrono::{Date, Utc};

extern crate jsonpath_lib as jsonpath;

lazy_static! {
    //Load balancing
    static ref SAVE_IN_PROCEES : Arc<RwLock<u8>> = Arc::new(RwLock::new(0));
    static ref KEYS_REM_EX_HASH : Arc<RwLock<HashMap<String, i64>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref DELETED_KEYS_LIST : Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));
    //Data
    static ref BTREE : Arc<RwLock<BTreeMap<String, ESRecord>>> = Arc::new(RwLock::new(BTreeMap::new()));
    static ref JSON_BTREE : Arc<RwLock<BTreeMap<String, Value>>> = Arc::new(RwLock::new(BTreeMap::new()));
    static ref GEO_BTREE : Arc<RwLock<BTreeMap<String, HashSet<GeoPoint2D>>>> = Arc::new(RwLock::new(BTreeMap::new()));
    static ref GEO_RTREE : Arc<RwLock<BTreeMap<String, RTree<GeoPoint2D>>>> = Arc::new(RwLock::new(BTreeMap::new()));
    //Time keepers
    static ref LAST_SAVE_TIME : Arc<RwLock<i64>> = Arc::new(RwLock::new(0));
    static ref LAST_SAVE_DURATION : Arc<RwLock<u64>> = Arc::new(RwLock::new(0));
    static ref MUTATION_COUNT_SINCE_SAVE : Arc<RwLock<u64>> = Arc::new(RwLock::new(0));
}


use rmp_serde;
use rmp_serde::encode::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::{OpenOptions, File};
use tokio::time::Instant;
use std::path::{PathBuf, Path};
use self::jsonpath::JsonPathError;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Database {
    btree: BTreeMap<String, ESRecord>,
    geo_tree: BTreeMap<String, HashSet<GeoPoint2D>>,
}

async fn load_db() {
    let path = match file_dirs::db_file_path() {
        Some(t) => t,
        None => { return; }
    };
    if !path.exists() {
        return;
    }
    let instant = Instant::now();

    let mut file = match OpenOptions::new().read(true).open(path).await {
        Ok(t) => t,
        Err(_) => { return; }
    };
    let mut content: Vec<u8> = vec![];
    let total_byte_read = match file.read_to_end(&mut content).await {
        Ok(t) => t,
        Err(_) => { return; }
    };

    debug!("Total data read {}", total_byte_read);

    let saved_db: Database = match rmp_serde::decode::from_read_ref(&content) {
        Ok(t) => t,
        Err(_) => { return; }
    };
    let mut btree: RwLockWriteGuard<BTreeMap<String, ESRecord>> = BTREE.write().unwrap();
    let mut geo_btree: RwLockWriteGuard<BTreeMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.write().unwrap();
    let mut r_map: RwLockWriteGuard<BTreeMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.write().unwrap();
    btree.clone_from(&saved_db.btree);
    geo_btree.clone_from(&saved_db.geo_tree);



    geo_btree.iter().for_each(|(k, v)| {
        let mut bulk_geo_hash_load: Vec<GeoPoint2D> = vec![];

        v.iter().for_each(|p| {
            bulk_geo_hash_load.push(p.clone())
        });

        r_map.insert(k.to_owned(), RTree::bulk_load(bulk_geo_hash_load));
    });

    let load_elapsed: Duration = instant.elapsed();
    info!("Database loaded from disk: {} seconds", load_elapsed.as_secs());
}

async fn save_db() {
    let mut btree_copy = BTreeMap::<String, ESRecord>::new();
    let mut geo_btree_copy = BTreeMap::<String, HashSet<GeoPoint2D>>::new();

    {
        let btree: RwLockReadGuard<BTreeMap<String, ESRecord>> = BTREE.read().unwrap();
        let geo_btree: RwLockReadGuard<BTreeMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.read().unwrap();
        btree_copy.clone_from(&btree);
        geo_btree_copy.clone_from(&geo_btree);
    }


    let db = Database {
        btree: btree_copy,
        geo_tree: geo_btree_copy,
    };

    let content = match rmp_serde::encode::to_vec(&db) {
        Ok(b) => { b }
        Err(e) => {
            error!("Error saving: {}", e);
            vec![]
        }
    };

    debug!("total db bytes: {}", content.len());
    let path = match file_dirs::db_file_path() {
        Some(t) => t,
        None => { return; }
    };
    let instant = Instant::now();

    let mut file = match OpenOptions::new().write(true).create(true).open(path).await {
        Ok(t) => t,
        Err(_) => { return; }
    };
    match file.write_all(&content).await {
        Ok(_) => {
            return;
        }
        Err(e) => {
            debug!("Error : {}", e);
            return;
        }
    };
}

pub async fn init_db() {
    lazy_static::initialize(&BTREE);
    lazy_static::initialize(&GEO_BTREE);
    lazy_static::initialize(&GEO_RTREE);
    lazy_static::initialize(&KEYS_REM_EX_HASH);
    lazy_static::initialize(&DELETED_KEYS_LIST);

    load_db().await;

    tokio::spawn(async {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            remove_expired_keys();
            let current_ts = Utc::now().timestamp();
            let map: RwLockReadGuard<HashMap<String, i64>> = KEYS_REM_EX_HASH.read().unwrap();
            if map.is_empty() {
                continue;
            }
            for (key, exp_time) in map.iter() {
                if exp_time.to_owned() <= current_ts {
                    let del_cmd = DelCmd {
                        arg_key: key.to_owned()
                    };
                    debug!("Remove Key -> {}", key);
                    del(&del_cmd);
                }
            }
        };
    });


    tokio::spawn(async {
        let mut interval = time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            let current_ts = Utc::now().timestamp();

            let mut map: RwLockWriteGuard<HashSet<String>> = DELETED_KEYS_LIST.write().unwrap();
            map.clear()
        };
    });


    tokio::spawn(async {
        let conf = crate::config::conf();
        let save_interval = conf.database.save_after as u64;
        let save_muts_cout = conf.database.mutations as u64;
        let mut interval = time::interval(Duration::from_secs(conf.database.save_after as u64));
        loop {
            interval.tick().await;
            let mut mutations = 0;
            {
                let mutation_count_since_save: RwLockReadGuard<u64> = MUTATION_COUNT_SINCE_SAVE.read().unwrap();
                mutations = *mutation_count_since_save;
            }

            let current_ts = Utc::now().timestamp();
            if save_muts_cout >= mutations {
                save_db().await;
            };
        };
    });
}

fn remove_expired_keys() {
    let map: RwLockReadGuard<HashSet<String>> = DELETED_KEYS_LIST.read().unwrap();
    let mut k_map: RwLockWriteGuard<HashMap<String, i64>> = KEYS_REM_EX_HASH.write().unwrap();
    map.iter().for_each(|key| {
        k_map.remove(key);
    });
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DataType {
    String,
    Integer,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ESRecord {
    pub key: String,
    pub value: String,
    pub data_type: DataType,
}

pub fn set(cmd: &SetCmd) -> String {
    //let arc: Arc<RwLock<BTreeMap<String, ESRecord>>> = BTREE;
    let mut map: RwLockWriteGuard<BTreeMap<String, ESRecord>> = BTREE.write().unwrap();


    let record = &ESRecord {
        key: cmd.arg_key.to_owned(),
        value: cmd.arg_value.to_owned(),
        data_type: cmd.arg_type.to_owned(),
    };

    if cmd.arg_exp > 0 {
        let timestamp = Utc::now().timestamp();
        let mut rem_map: RwLockWriteGuard<HashMap<String, i64>> = KEYS_REM_EX_HASH.write().unwrap();
        rem_map.insert(cmd.arg_key.to_owned(), (cmd.arg_exp.to_owned() as i64 + timestamp));
    }

    map.insert(record.key.to_owned(), record.to_owned());

    print_ok()
}

pub fn get(cmd: &GetCmd) -> String {
    let map: RwLockReadGuard<BTreeMap<String, ESRecord>> = BTREE.read().unwrap();
    let key = &cmd.arg_key.to_owned();


    return match map.get(key) {
        Some(r) => {
            print_record(r)
        }
        None => {
            print_err("KEY_NOT_FOUND")
        }
    };
}

pub fn exists(cmd: &ExistsCmd) -> String {
    let map: RwLockReadGuard<BTreeMap<String, ESRecord>> = BTREE.read().unwrap();

    let mut found_count: i64 = 0;

    for key in &cmd.keys {
        if map.contains_key(key) {
            found_count += 1;
        }
    }

    print_integer(found_count)
}

pub fn info(cmd: &InfoCmd) -> String {
    let map: RwLockReadGuard<BTreeMap<String, ESRecord>> = BTREE.read().unwrap();
    let key_count = map.keys().count();
    let info = format!("db0:keys={}\r\n", key_count);
    print_string(&info)
}

pub fn del(cmd: &DelCmd) -> String {
    let mut map: RwLockWriteGuard<BTreeMap<String, ESRecord>> = BTREE.write().unwrap();
    let key = &cmd.arg_key.to_owned();
    return match map.remove(key) {
        Some(r) => {
            let mut map: RwLockWriteGuard<HashSet<String>> = DELETED_KEYS_LIST.write().unwrap();
            map.insert(key.to_owned());
            print_ok()
        }
        None => {
            print_err("KEY_NOT_FOUND")
        }
    };
}

pub fn keys(cmd: &KeysCmd) -> String {
    let map: RwLockReadGuard<BTreeMap<String, ESRecord>> = BTREE.read().unwrap();
    let pattern_marcher = match Pattern::new(&cmd.pattern) {
        Ok(t) => t,
        Err(e) => {
            return print_err("ERR invalid pattern");
        }
    };

    let mut keys: Vec<&String> = vec![];

    for key in map.keys() {
        if pattern_marcher.matches(key) {
            keys.push(key)
        }
    }
    print_string_arr(keys)
}

pub fn geo_add(cmd: &GeoAddCmd) -> String {
    let mut r_map: RwLockWriteGuard<BTreeMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.write().unwrap();

    let mut map: RwLockWriteGuard<BTreeMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.write().unwrap();


    let mut point_map: HashSet<GeoPoint2D> = HashSet::new();
    if map.contains_key(&cmd.arg_key) {
        //update previous insertion
        let p = map.get_mut(&cmd.arg_key).unwrap();
        point_map = point_map.union(p).cloned().collect();
    }

    let mut is_valid_geo_point = true;
    let mut invalid_geo_point_msg: String = String::new();

    cmd.items.iter().for_each(|(lat, lng, tag)| {
        let tag = tag.to_owned();
        let lat = lat.to_owned();
        let lng = lng.to_owned();


        let hash = match geohash::encode(Coordinate { x: lng, y: lat }, 10) {
            Ok(t) => t,
            Err(e) => {
                is_valid_geo_point = false;
                invalid_geo_point_msg = format!("{}", e);
                return;
            }
        };

        if !is_valid_geo_point {
            return;
        }

        let point = GeoPoint2D {
            tag,
            x_cord: lat,
            y_cord: lng,
            hash,
        };
        point_map.insert(point);
    });

    if !is_valid_geo_point {
        let mut msg = String::from("ERR ");
        msg += &invalid_geo_point_msg;
        return print_err(&msg);
    }


    let mut bulk_geo_hash_load: Vec<GeoPoint2D> = vec![];

    point_map.iter().for_each(|p| {
        bulk_geo_hash_load.push(p.clone())
    });

    map.insert(cmd.arg_key.to_owned(), point_map);
    r_map.insert(cmd.arg_key.to_owned(), RTree::bulk_load(bulk_geo_hash_load));

    print_ok()
}

pub fn geo_hash(cmd: &GeoHashCmd) -> String {
    let map: RwLockReadGuard<BTreeMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.read().unwrap();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();
    let empty_string = String::new();

    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m,
        None => {
            return print_err("KEY_NOT_FOUND");
        }
    };

    let mut geo_hashes: Vec<&String> = vec![];

    for s in &cmd.items {
        let test_geo = GeoPoint2D {
            tag: s.to_owned(),
            x_cord: 0.0,
            y_cord: 0.0,
            hash: "".to_string(),
        };
        match geo_point_hash_set.get(&test_geo) {
            Some(t) => {
                geo_hashes.push(&t.hash)
            }
            None => {
                geo_hashes.push(&empty_string)
            }
        };
    }

    print_string_arr(geo_hashes)
}

pub fn geo_dist(cmd: &GeoDistCmd) -> String {
    let map: RwLockReadGuard<BTreeMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.read().unwrap();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();


    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m,
        None => {
            return print_err("KEY_NOT_FOUND");
        }
    };
    let comp = GeoPoint2D {
        tag: cmd.arg_mem_1.to_owned(),
        x_cord: 0.0,
        y_cord: 0.0,
        hash: "".to_string(),
    };
    let member_1 = match geo_point_hash_set.get(&comp) {
        Some(t) => {
            t
        }
        None => {
            return print_err("ERR member 1 not found");
        }
    };
    let comp = GeoPoint2D {
        tag: cmd.arg_mem_2.to_owned(),
        x_cord: 0.0,
        y_cord: 0.0,
        hash: "".to_string(),
    };
    let member_2 = match geo_point_hash_set.get(&comp) {
        Some(t) => {
            t
        }
        None => {
            return print_err("ERR member 2 not found");
        }
    };

    let distance = util::haversine_distance(Location { latitude: member_1.x_cord, longitude: member_1.y_cord },
                                            Location { latitude: member_2.x_cord, longitude: member_2.y_cord },
                                            cmd.arg_unit.clone());
    print_string(&distance.to_string())
}

pub fn geo_radius(cmd: &GeoRadiusCmd) -> String {
    let r_map: RwLockReadGuard<BTreeMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.read().unwrap();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();

    let geo_points_rtree = match r_map.get(&cmd.arg_key) {
        Some(m) => m,
        None => {
            return print_err("KEY_NOT_FOUND");
        }
    };

    let radius = match cmd.arg_unit {
        Units::Kilometers => km_m(cmd.arg_radius),
        Units::Miles => mi_m(cmd.arg_radius),
        Units::Meters => cmd.arg_radius,
    };

    let circle = Circle {
        origin: [cmd.arg_lat, cmd.arg_lng],
        radius,
    };

    /*
       ["Palermo","190.4424","st0219xsd21"]
    */

    let nearest_in_radius_array = &mut geo_points_rtree.nearest_neighbor_iter_with_distance(&circle.origin);

    let mut item_string_arr: Vec<Vec<String>> = vec![];

    while let Some((point, dist)) = nearest_in_radius_array.next() {
        if dist <= circle.radius {
            let dist = match cmd.arg_unit {
                Units::Kilometers => m_km(dist),
                Units::Miles => m_mi(dist),
                Units::Meters => dist,
            };

            let string_arr: Vec<String> = vec![point.tag.to_owned(), point.hash.to_owned(), dist.to_string()];
            &item_string_arr.push(string_arr);
        }
    }
    match cmd.arg_order {
        ArgOrder::UNSPECIFIED => (),
        ArgOrder::ASC => item_string_arr.sort_by(|a, b| a[2].cmp(&b[2])),
        ArgOrder::DESC => item_string_arr.sort_by(|a, b| b[2].cmp(&a[2]))
    };


    print_nested_arr(item_string_arr)
}

pub fn geo_radius_by_member(cmd: &GeoRadiusByMemberCmd) -> String {
    let map: RwLockReadGuard<BTreeMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.read().unwrap();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();


    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m,
        None => {
            return print_err("KEY_NOT_FOUND");
        }
    };

    let comp = GeoPoint2D {
        tag: cmd.member.to_owned(),
        x_cord: 0.0,
        y_cord: 0.0,
        hash: "".to_string(),
    };
    let member = match geo_point_hash_set.get(&comp) {
        Some(t) => {
            t
        }
        None => {
            return print_err("ERR member 1 not found");
        }
    };

    let cmd = GeoRadiusCmd {
        arg_key: cmd.arg_key.to_owned(),
        arg_lng: member.y_cord,
        arg_lat: member.x_cord,
        arg_radius: cmd.arg_radius,
        arg_unit: cmd.arg_unit,
        arg_order: cmd.arg_order,
    };

    geo_radius(&cmd)
}


pub fn geo_pos(cmd: &GeoPosCmd) -> String {
    let map: RwLockReadGuard<BTreeMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.read().unwrap();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();


    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m,
        None => {
            return print_err("KEY_NOT_FOUND");
        }
    };

    let mut points_array: Vec<Vec<String>> = vec![];

    for s in &cmd.items {
        let test_geo = GeoPoint2D {
            tag: s.to_owned(),
            x_cord: 0.0,
            y_cord: 0.0,
            hash: "".to_string(),
        };
        match geo_point_hash_set.get(&test_geo) {
            Some(t) => {
                let point_array: Vec<String> = vec![t.x_cord.to_string(), t.y_cord.to_string()];
                points_array.push(point_array)
            }
            None => {
                points_array.push(vec![])
            }
        };
    }

    print_nested_arr(points_array)
}

pub fn geo_del(cmd: &GeoDelCmd) -> String {
    let mut r_map: RwLockWriteGuard<BTreeMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.write().unwrap();
    let mut map: RwLockWriteGuard<BTreeMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.write().unwrap();

    if !(r_map.contains_key(&cmd.arg_key) && map.contains_key(&cmd.arg_key)) {
        return print_err("KEY_NOT_FOUND");
    }
    r_map.remove(&cmd.arg_key);
    map.remove(&cmd.arg_key);

    print_ok()
}

pub fn geo_remove(cmd: &GeoRemoveCmd) -> String {
    let mut r_map: RwLockWriteGuard<BTreeMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.write().unwrap();

    let mut map: RwLockWriteGuard<BTreeMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.write().unwrap();

    if !(r_map.contains_key(&cmd.arg_key) && map.contains_key(&cmd.arg_key)) {
        return print_err("KEY_NOT_FOUND");
    }
    let geo_point_hash_set = match map.get_mut(&cmd.arg_key) {
        Some(m) => m,
        None => {
            return print_err("KEY_NOT_FOUND");
        }
    };

    for s in &cmd.items {
        let comp = GeoPoint2D {
            tag: s.to_owned(),
            x_cord: 0.0,
            y_cord: 0.0,
            hash: "".to_string(),
        };

        geo_point_hash_set.remove(&comp);
    }

    if geo_point_hash_set.is_empty() {
        map.remove(&cmd.arg_key);
        r_map.remove(&cmd.arg_key);
        return print_ok();
    }

    let mut bulk_geo_hash_load: Vec<GeoPoint2D> = vec![];
    let mut point_map: HashSet<GeoPoint2D> = HashSet::new();

    geo_point_hash_set.iter().for_each(|p| {
        bulk_geo_hash_load.push(p.clone())
    });

    point_map = point_map.union(geo_point_hash_set).cloned().collect();


    map.insert(cmd.arg_key.to_owned(), point_map);
    r_map.insert(cmd.arg_key.to_owned(), RTree::bulk_load(bulk_geo_hash_load));

    print_ok()
}

pub fn geo_json(cmd: &GeoJsonCmd) -> String {
    let map: RwLockReadGuard<BTreeMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.read().unwrap();

    let empty_string = String::new();

    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m,
        None => {
            return print_err("KEY_NOT_FOUND");
        }
    };

    let mut geo_arr: Vec<GeoPoint2D> = vec![];

    for s in &cmd.items {
        let test_geo = GeoPoint2D {
            tag: s.to_owned(),
            x_cord: 0.0,
            y_cord: 0.0,
            hash: "".to_string(),
        };
        match geo_point_hash_set.get(&test_geo) {
            Some(t) => {
                geo_arr.push(t.to_owned())
            }
            None => {}
        };
    }

    print_string(&build_geo_json(&geo_arr).to_string())
}

// JSET, JGET, JDEL, JPATH, JMERGE
pub fn jset(cmd: &JSetCmd) -> String {
    let mut map: RwLockWriteGuard<BTreeMap<String, Value>> = JSON_BTREE.write().unwrap();


    let value: Value = match serde_json::from_str(&cmd.arg_value) {
        Ok(t) => t,
        Err(_) => { return print_err("ERR invalid json"); }
    };

    map.insert(cmd.arg_key.to_owned(), value);

    print_ok()
}

pub fn jmerge(cmd: &JMergeCmd) -> String {
    let null_value = Value::Null;
    let mut map: RwLockWriteGuard<BTreeMap<String, Value>> = JSON_BTREE.write().unwrap();

    let mut value: Value = match serde_json::from_str(&cmd.arg_value) {
        Ok(t) => t,
        Err(_) => { return print_err("ERR invalid json"); }
    };

    let prev_value = match map.get(&cmd.arg_key) {
        None => { &null_value }
        Some(v) => { v }
    };

    if prev_value.is_null() {
        map.insert(cmd.arg_key.to_owned(), value);
        return print_ok();
    }

    util::merge(&mut value, prev_value);
    map.insert(cmd.arg_key.to_owned(), value);
    print_ok()
}

pub fn jget(cmd: &JGetCmd) -> String {
    let null_value = Value::Null;
    let map: RwLockReadGuard<BTreeMap<String, Value>> = JSON_BTREE.read().unwrap();

    let value = match map.get(&cmd.arg_key) {
        None => { &null_value }
        Some(v) => { v }
    };

    if value.is_null() {
        return print_string(&"".to_owned());
    }
    print_string(&value.to_string())
}

pub fn jpath(cmd: &JPathCmd) -> String {
    let null_value = Value::Null;
    let map: RwLockReadGuard<BTreeMap<String, Value>> = JSON_BTREE.read().unwrap();

    let value = match map.get(&cmd.arg_key) {
        None => { &null_value }
        Some(v) => { v }
    };

    if value.is_null() {
        return print_string(&"".to_owned());
    }
    let json_result = match jsonpath::select(value, cmd.arg_selector.as_str()) {
        Ok(v) => { v }
        Err(_) => { return print_string(&String::from("")); }
    };

    let mut j_strings : Vec<String> = vec![];

    for v in json_result {
        j_strings.push(v.to_owned().to_string())
    }
    //let selected = json!(json_result);
    print_arr(j_strings)
}

pub fn jdel(cmd: &JDelCmd) -> String {
    let null_value = Value::Null;
    let mut map: RwLockWriteGuard<BTreeMap<String, Value>> = JSON_BTREE.write().unwrap();
    map.remove(&cmd.arg_key);
    print_ok()
}