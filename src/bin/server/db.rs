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
extern crate dashmap;

use serde_json::Value;
use tokio::time;
use std::time::{Duration, SystemTime};
use chrono::{Date, Utc};

extern crate jsonpath_lib as jsonpath;
extern crate json_dotpath;


use rmp_serde;
use rmp_serde::encode::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::{OpenOptions, File};
use tokio::time::Instant;
use std::path::{PathBuf, Path};
use self::jsonpath::JsonPathError;
use self::dashmap::{DashMap, DashSet};
use regex::internal::Input;
use self::dashmap::mapref::one::{RefMut, Ref};
use json_dotpath::DotPaths;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ESValue {
    String(String),
    Int(i64),
}

impl ESValue {
    fn as_int(&self) -> Option<&i64> {
        match self {
            ESValue::String(_) => {
                None
            }
            ESValue::Int(i) => {
                Some(i)
            }
        }
    }
    fn as_string(&self) -> Option<&String> {
        match self {
            ESValue::String(s) => {
                Some(s)
            }
            ESValue::Int(_) => {
                None
            }
        }
    }
}

lazy_static! {
    //Load balancing
    static ref SAVE_IN_PROCEES : Arc<RwLock<u8>> = Arc::new(RwLock::new(0));
    static ref KEYS_REM_EX_HASH : Arc<DashMap<String, i64>> = Arc::new(DashMap::new());
    static ref DELETED_KEYS_LIST : Arc<DashSet<String>> = Arc::new(DashSet::new());
    //Data
    static ref BTREE : Arc<DashMap<String, ESValue>> = Arc::new(DashMap::new());
    static ref JSON_BTREE : Arc<DashMap<String, Value>> = Arc::new(DashMap::new());
    static ref GEO_BTREE : Arc<DashMap<String, HashSet<GeoPoint2D>>> = Arc::new(DashMap::new());
    static ref GEO_RTREE : Arc<DashMap<String, RTree<GeoPoint2D>>> = Arc::new(DashMap::new());
    //Time keepers
    static ref LAST_SAVE_TIME : Arc<RwLock<i64>> = Arc::new(RwLock::new(0));
    static ref LAST_SAVE_DURATION : Arc<RwLock<u64>> = Arc::new(RwLock::new(0));
    static ref MUTATION_COUNT_SINCE_SAVE : Arc<RwLock<usize>> = Arc::new(RwLock::new(0));
}


#[derive(Clone, Debug, Serialize, Deserialize)]
struct Database {
    btree: DashMap<String, ESValue>,
    json_btree: DashMap<String, Value>,
    geo_tree: DashMap<String, HashSet<GeoPoint2D>>,
}

fn increment_mutation_counter() {
    let mut mutation_counter: RwLockWriteGuard<usize> = MUTATION_COUNT_SINCE_SAVE.write().unwrap();
    *mutation_counter += 1;
}

fn increment_mutation_counter_by(u: usize) {
    let mut mutation_counter: RwLockWriteGuard<usize> = MUTATION_COUNT_SINCE_SAVE.write().unwrap();
    *mutation_counter += u;
}

fn reset_mutation_counter() {
    let mut mutation_counter: RwLockWriteGuard<usize> = MUTATION_COUNT_SINCE_SAVE.write().unwrap();
    *mutation_counter = 0;
}


async fn load_db() {
    let path = match file_dirs::db_file_path() {
        Some(t) => t,
        None => { return; }
    };
    if !path.exists() {
        return;
    }

    info!("Loading DB file: {}", path.as_os_str().to_str().unwrap());

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
    let mut btree: Arc<DashMap<String, ESValue>> = BTREE.clone();
    let mut json_btree: Arc<DashMap<String, Value>> = JSON_BTREE.clone();
    let mut geo_btree: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();
    let mut r_map: Arc<DashMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.clone();

    // geo_btree.clone_from(&saved_db.geo_tree);

    &saved_db.geo_tree.iter().for_each(|data| {
        geo_btree.insert(data.key().to_owned(), data.value().to_owned());
    }
    );

    &saved_db.json_btree.iter().for_each(|data| {
        json_btree.insert(data.key().to_owned(), data.value().to_owned());
    }
    );

    &saved_db.btree.iter().for_each(|data| {
        btree.insert(data.key().to_owned(), data.value().to_owned());
    }
    );

    geo_btree.iter().for_each(|data| {
        let mut bulk_geo_hash_load: Vec<GeoPoint2D> = vec![];

        data.value().iter().for_each(|p| {
            bulk_geo_hash_load.push(p.clone())
        });

        r_map.insert(data.key().to_owned(), RTree::bulk_load(bulk_geo_hash_load));
    });

    let load_elapsed: Duration = instant.elapsed();
    info!("Database loaded from disk: {} seconds", load_elapsed.as_secs());
}

async fn save_db() {
    let mut json_btree_copy = DashMap::<String, Value>::new();
    let mut btree_copy = DashMap::<String, ESValue>::new();
    let mut geo_btree_copy = DashMap::<String, HashSet<GeoPoint2D>>::new();

    {
        let json_btree: Arc<DashMap<String, Value>> = JSON_BTREE.clone();
        let btree: Arc<DashMap<String, ESValue>> = BTREE.clone();
        let geo_btree: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();

        json_btree_copy.clone_from(&json_btree);
        btree_copy.clone_from(&btree);
        geo_btree_copy.clone_from(&geo_btree);
    }


    let db = Database {
        btree: btree_copy,
        geo_tree: geo_btree_copy,
        json_btree: json_btree_copy,
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
            reset_mutation_counter();
            let mut last_save_time: RwLockWriteGuard<i64> = LAST_SAVE_TIME.write().unwrap();
            *last_save_time = Utc::now().timestamp();
            return;
        }
        Err(e) => {
            debug!("Error : {}", e);
            return;
        }
    };
}

extern crate rayon;
use rayon::prelude::*;

pub async fn init_db() {
    lazy_static::initialize(&BTREE);
    lazy_static::initialize(&JSON_BTREE);
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

            let mutation_count_since_save: RwLockReadGuard<usize> = MUTATION_COUNT_SINCE_SAVE.read().unwrap();
            debug!("Mutation Count: {}", *mutation_count_since_save);
            std::mem::drop(mutation_count_since_save);

            let current_ts = Utc::now().timestamp();
            let mut map: Arc<DashMap<String, i64>> = KEYS_REM_EX_HASH.clone();
            //let map = map.into_read_only();
            if map.is_empty() {
                continue;
            }

            map.iter().par_bridge().for_each(|data| {
                let exp_time = data.value();
                let key = data.key();
                if exp_time.to_owned() <= current_ts {
                    debug!("Remove Key -> {}", key);
                    del(&DelCmd {
                        arg_key: key.to_owned()
                    });
                }
            });
        };
    });


    tokio::spawn(async {
        let mut interval = time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            let current_ts = Utc::now().timestamp();

            let mut map: Arc<DashSet<String>> = DELETED_KEYS_LIST.clone();
            map.clear()
        };
    });


    tokio::spawn(async {
        let conf = crate::config::conf();
        let save_interval = conf.database.save_after as u64;
        let save_muts_cout = conf.database.mutations;
        let mut interval = time::interval(Duration::from_secs(conf.database.save_after as u64));
        loop {
            interval.tick().await;
            let mut mutations = 0;
            {
                let mutation_count_since_save: RwLockReadGuard<usize> = MUTATION_COUNT_SINCE_SAVE.read().unwrap();
                mutations = *mutation_count_since_save;
            }

            let current_ts = Utc::now().timestamp();
            if mutations >= save_muts_cout {
                save_db().await;
            };
        };
    });
}

fn clear_db() {
    let mut b_map: Arc<DashMap<String, ESValue>> = BTREE.clone();
    let mut k_map: Arc<DashMap<String, i64>> = KEYS_REM_EX_HASH.clone();
    let mut deleted_keys_map: Arc<DashSet<String>> = DELETED_KEYS_LIST.clone();
    let mut r_map: Arc<DashMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.clone();
    let mut geo_map: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();
    let mut json_map: Arc<DashMap<String, Value>> = JSON_BTREE.clone();

    increment_mutation_counter_by(b_map.len());
    increment_mutation_counter_by(k_map.len());
    increment_mutation_counter_by(r_map.len());
    increment_mutation_counter_by(geo_map.len());
    increment_mutation_counter_by(json_map.len());
    //b_map.len();

    b_map.clear();
    k_map.clear();
    deleted_keys_map.clear();
    r_map.clear();
    geo_map.clear();
    json_map.clear();
}

fn remove_expired_keys() {
    let map: Arc<DashSet<String>> = DELETED_KEYS_LIST.clone();
    let mut k_map: Arc<DashMap<String, i64>> = KEYS_REM_EX_HASH.clone();
    map.iter().for_each(|data| {
        k_map.remove(data.key());
    });
}



pub fn last_save(cmd: &LastSaveCmd) -> String {
    //let arc: Arc<RwLock<BTreeMap<String, ESRecord>>> = BTREE;
    let last_save_time: RwLockReadGuard<i64> = LAST_SAVE_TIME.read().unwrap();
    print_integer(*last_save_time)
}

pub fn bg_save(cmd: &BGSaveCmd) -> String {
    tokio::task::spawn(async {
        save_db();
    });
    print_ok()
}

pub fn flush_db(cmd: &FlushDBCmd) -> String {
    tokio::task::spawn(async {
        clear_db();
    });
    print_ok()
}


pub fn set(cmd: &SetCmd) -> String {
    //let arc: Arc<RwLock<BTreeMap<String, ESRecord>>> = BTREE;
    let mut map: Arc<DashMap<String, ESValue>> = BTREE.clone();


    if cmd.arg_exp > 0 {
        let timestamp = Utc::now().timestamp();
        let mut rem_map: Arc<DashMap<String, i64>> = KEYS_REM_EX_HASH.clone();
        rem_map.insert(cmd.arg_key.to_owned(), (cmd.arg_exp.to_owned() as i64 + timestamp));
    }

    map.insert(cmd.arg_key.to_owned(), cmd.arg_value.to_owned());

    increment_mutation_counter();
    print_ok()
}

pub fn get(cmd: &GetCmd) -> String {
    let map: Arc<DashMap<String, ESValue>> = BTREE.clone();
    let key = &cmd.arg_key.to_owned();

    return match map.get(key) {
        Some(r) => {
            match r.value() {
                ESValue::String(s) => {
                    print_string(s)
                }
                ESValue::Int(i) => {
                    print_integer(i.to_owned())
                }
            }
            //print_record(r.value())
        }
        None => {
            print_err("KEY_NOT_FOUND")
        }
    };
}

pub fn exists(cmd: &ExistsCmd) -> String {
    let map: Arc<DashMap<String, ESValue>> = BTREE.clone();

    let mut found_count: i64 = 0;
    for key in &cmd.keys {
        if map.contains_key(key) {
            found_count += 1;
        }
    }

    print_integer(found_count)
}

pub fn info(cmd: &InfoCmd) -> String {
    let map: Arc<DashMap<String, ESValue>> = BTREE.clone();
    //let map = map.into_read_only();
    let key_count = map.len();
    let info = format!("db0:keys={}\r\n", key_count);
    print_string(&info)
}

pub fn del(cmd: &DelCmd) -> String {
    let mut map: Arc<DashMap<String, ESValue>> = BTREE.clone();
    let key = &cmd.arg_key.to_owned();
    return match map.remove(key) {
        Some(r) => {
            let mut map: Arc<DashSet<String>> = DELETED_KEYS_LIST.clone();
            map.insert(key.to_owned());
            increment_mutation_counter();
            print_ok()
        }
        None => {
            print_err("KEY_NOT_FOUND")
        }
    };
}

pub fn persist(cmd: &PersistCmd) -> String {
    let mut map: Arc<DashMap<String, i64>> = KEYS_REM_EX_HASH.clone();
    let key = &cmd.arg_key;

    return match map.remove(key) {
        None => {
            print_integer(0)
        }
        Some(_) => {
            print_integer(1)
        }
    };
}

pub fn ttl(cmd: &TTLCmd) -> String {
    let rem_map: Arc<DashMap<String, i64>> = KEYS_REM_EX_HASH.clone();
    let b_map: Arc<DashMap<String, ESValue>> = BTREE.clone();
    let key: &String = &cmd.arg_key;

    let mut out = 0;

    if !b_map.contains_key(key) {
        out += -1
    }
    return match rem_map.get(key) {
        None => {
            out += -1;
            print_integer(out)
        }
        Some(data) => {
            let ttl = data.value();
            print_integer(*ttl)
        }
    };
}

pub fn expire(cmd: &ExpireCmd) -> String {
    let mut rem_map: Arc<DashMap<String, i64>> = KEYS_REM_EX_HASH.clone();
    let b_map: Arc<DashMap<String, ESValue>> = BTREE.clone();
    let key: String = cmd.arg_key.to_owned();
    let value: i64 = cmd.arg_value;

    let mut out = 0;

    if !b_map.contains_key(&key) {
        return print_integer(out);
    }

    let expire_at = Utc::now().timestamp() + value;

    rem_map.insert(key, expire_at);

    print_integer(out)
}

pub fn expire_at(cmd: &ExpireAtCmd) -> String {
    let mut rem_map: Arc<DashMap<String, i64>> = KEYS_REM_EX_HASH.clone();
    let b_map: Arc<DashMap<String, ESValue>> = BTREE.clone();
    let key: String = cmd.arg_key.to_owned();
    let expire_at: i64 = cmd.arg_value;

    let mut out = 0;

    if !b_map.contains_key(&key) {
        return print_integer(out);
    }

    rem_map.insert(key, expire_at);

    print_integer(out)
}

pub fn incr_by(cmd: &ExpireCmd) -> String {
    let mut b_map: Arc<DashMap<String, ESValue>> = BTREE.clone();
    let key: String = cmd.arg_key.to_owned();
    let value: i64 = cmd.arg_value;
    let default_v = 0;

    let result = match b_map.get_mut(&key) {
        None => {
            let value = ESValue::Int(value);
            b_map.insert(key, value.clone());
            return print_integer(*value.as_int().unwrap());
        }
        Some(mut data) => {
            match data.value_mut() {
                ESValue::String(s) => {
                    match s.parse::<i64>() {
                        Ok(d) => {
                            let new_value = d + value;
                            *s = new_value.to_string();
                            print_string(s)
                        }
                        Err(_) => {
                            print_err("ERR string cannot be represented as integer")
                        }
                    }
                }
                ESValue::Int(i) => {
                    *i += value;
                    print_integer(*i)
                }
            }
        }
    };

    result
}

pub fn keys(cmd: &KeysCmd) -> String {
    let map: Arc<DashMap<String, ESValue>> = BTREE.clone();
    //let map = map.into_read_only();
    let pattern_marcher = match Pattern::new(&cmd.pattern) {
        Ok(t) => t,
        Err(e) => {
            return print_err("ERR invalid pattern");
        }
    };

    let mut keys: Vec<String> = vec![];

    for item in map.iter() {
        //let key = .to_owned();

        if pattern_marcher.matches(item.key()) {
            keys.push(item.key().clone())
        }
    }

    print_arr(keys)
}

pub fn geo_add(cmd: &GeoAddCmd) -> String {
    let mut r_map: Arc<DashMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.clone();

    let mut map: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();


    let mut point_map: HashSet<GeoPoint2D> = HashSet::new();
    if map.contains_key(&cmd.arg_key) {
        //update previous insertion
        let p = map.get_mut(&cmd.arg_key).unwrap();
        point_map = point_map.union(p.value()).cloned().collect();
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

    increment_mutation_counter();
    print_ok()
}

pub fn geo_hash(cmd: &GeoHashCmd) -> String {
    let map: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();
    let empty_string = String::new();

    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m.value().to_owned(),
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
    let map: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();


    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m.value().to_owned(),
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
    let r_map: Arc<DashMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.clone();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();

    let geo_points_rtree = match r_map.get(&cmd.arg_key) {
        Some(m) => m.value().to_owned(),
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
    let map: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();


    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m.value().to_owned(),
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
    let map: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();


    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m.value().to_owned(),
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
    let mut r_map: Arc<DashMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.clone();
    let mut map: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();

    if !(r_map.contains_key(&cmd.arg_key) && map.contains_key(&cmd.arg_key)) {
        return print_err("KEY_NOT_FOUND");
    }
    r_map.remove(&cmd.arg_key);
    map.remove(&cmd.arg_key);

    increment_mutation_counter();
    print_ok()
}

pub fn geo_remove(cmd: &GeoRemoveCmd) -> String {
    let mut r_map: Arc<DashMap<String, RTree<GeoPoint2D>>> = GEO_RTREE.clone();

    let mut map: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();

    if !(r_map.contains_key(&cmd.arg_key) && map.contains_key(&cmd.arg_key)) {
        return print_err("KEY_NOT_FOUND");
    }
    let mut geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m.value().to_owned(),
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
        increment_mutation_counter();
        return print_ok();
    }

    let mut bulk_geo_hash_load: Vec<GeoPoint2D> = vec![];
    let mut point_map: HashSet<GeoPoint2D> = HashSet::new();

    geo_point_hash_set.iter().for_each(|p| {
        bulk_geo_hash_load.push(p.clone())
    });

    point_map = point_map.union(&geo_point_hash_set).cloned().collect();


    map.insert(cmd.arg_key.to_owned(), point_map);
    r_map.insert(cmd.arg_key.to_owned(), RTree::bulk_load(bulk_geo_hash_load));
    increment_mutation_counter();
    print_ok()
}

pub fn geo_json(cmd: &GeoJsonCmd) -> String {
    let map: Arc<DashMap<String, HashSet<GeoPoint2D>>> = GEO_BTREE.clone();

    let empty_string = String::new();

    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m.value().to_owned(),
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
pub fn jset_raw(cmd: &JSetRawCmd) -> String {
    let mut map: Arc<DashMap<String, Value>> = JSON_BTREE.clone();


    let json_value: Value = match serde_json::from_str(&cmd.arg_value) {
        Ok(t) => t,
        Err(_) => { return print_err("ERR invalid json"); }
    };

    map.insert(cmd.arg_key.to_owned(), json_value);
    increment_mutation_counter();
    print_ok()
}

pub fn jset(cmd: &JSetCmd) -> String {
    let mut map: Arc<DashMap<String, Value>> = JSON_BTREE.clone();

    return match map.get_mut(&cmd.arg_key) {
        None => {
            let mut ers: Vec<json_dotpath::Error> = vec![];
            let mut json = Value::Null;
            for (path, value) in &cmd.arg_set_items {
                match json.dot_set(path, value.to_owned()) {
                    Ok(t) => {}
                    Err(e) => {
                        ers.push(e)
                    }
                };
            }
            if !ers.is_empty() {
                return print_err("Error Saving values");
            }
            map.insert(cmd.arg_key.to_owned(), json);
            increment_mutation_counter();
            return print_ok();
        }
        Some(mut j) => {
            let mut ers: Vec<json_dotpath::Error> = vec![];
            let mut json = j.value_mut();
            for (path, value) in &cmd.arg_set_items {
                //json.dot_set(&cmd.arg_dot_path, cmd.arg_json_value.clone());
                match json.dot_set(&path, value.to_owned()) {
                    Ok(t) => {}
                    Err(e) => {
                        ers.push(e)
                    }
                };
            }
            if !ers.is_empty() {
                return print_err("Error some values");
            }
            let string = j.to_string();
            increment_mutation_counter();
            print_ok()
        }
    };
}

pub fn jmerge(cmd: &JMergeCmd) -> String {
    let null_value = Value::Null;
    let mut map: Arc<DashMap<String, Value>> = JSON_BTREE.clone();

    let mut value: Value = match serde_json::from_str(&cmd.arg_value) {
        Ok(t) => t,
        Err(_) => { return print_err("ERR invalid json"); }
    };

    let prev_value: Value = match map.get(&cmd.arg_key) {
        None => { null_value }
        Some(v) => { v.value().to_owned() }
    };

    if prev_value.is_null() {
        map.insert(cmd.arg_key.to_owned(), value);
        increment_mutation_counter();
        return print_ok();
    }

    util::merge(&mut value, &prev_value);
    map.insert(cmd.arg_key.to_owned(), value);
    increment_mutation_counter();
    print_ok()
}

pub fn jget(cmd: &JGetCmd) -> String {
    let null_value = Value::Null;
    let map: Arc<DashMap<String, Value>> = JSON_BTREE.clone();

    let value: Value = match map.get(&cmd.arg_key) {
        None => { null_value }
        Some(v) => { v.value().to_owned() }
    };

    if value.is_null() {
        return print_string(&"".to_owned());
    }
    if let Some(t) = &cmd.arg_dot_path {
        let dot_path_value = value.dot_get::<Value>(t).unwrap_or(Some(Value::Null)).unwrap();
        return match dot_path_value {
            Value::String(s) => {
                print_string(&s)
            }
            Value::Number(n) => {
                print_string(&n.to_string())
            }
            v => {
                print_string(&v.to_string())
            }
        };
    }
    print_string(&value.to_string())
}

pub fn jpath(cmd: &JPathCmd) -> String {
    let null_value = Value::Null;
    let map: Arc<DashMap<String, Value>> = JSON_BTREE.clone();

    let value = match map.get(&cmd.arg_key) {
        None => { null_value }
        Some(v) => { v.value().to_owned() }
    };

    if value.is_null() {
        return print_string(&"".to_owned());
    }
    let json_result = match jsonpath::select(&value, cmd.arg_selector.as_str()) {
        Ok(v) => { v }
        Err(_) => { return print_string(&String::from("")); }
    };

    let mut j_strings: Vec<String> = vec![];

    for v in json_result {
        j_strings.push(v.to_owned().to_string())
    }
    //let selected = json!(json_result);
    print_arr(j_strings)
}

pub fn jdel(cmd: &JDelCmd) -> String {
    let null_value = Value::Null;
    let mut map: Arc<DashMap<String, Value>> = JSON_BTREE.clone();
    map.remove(&cmd.arg_key);
    print_ok()
}


pub fn jincr_by(cmd: &JIncrByCmd) -> String {
    let mut map: Arc<DashMap<String, Value>> = JSON_BTREE.clone();
    return match map.get_mut(&cmd.arg_key) {
        None => {
            return print_err("ERR key not found");
        }
        Some(mut j) => {
            let mut json = j.value_mut();
            let mut path_to_incr = json.dot_get(&cmd.arg_path).unwrap_or(Some(Value::Null)).unwrap_or(Value::Null);

            if path_to_incr.is_null() {
                let new_value = json!(cmd.arg_increment_value);
                json.dot_set(&cmd.arg_path.to_owned(), new_value.clone());
                increment_mutation_counter();
                return print_integer(new_value.as_i64().unwrap());
            }
            let new_value = if path_to_incr.is_number() {
                if path_to_incr.is_i64() {
                    let inc = path_to_incr.as_i64().unwrap() + cmd.arg_increment_value;
                    json!(inc)
                } else if path_to_incr.is_f64() {
                    let inc = path_to_incr.as_f64().unwrap() + (cmd.arg_increment_value as f64);
                    json!(inc)
                } else if path_to_incr.is_u64() {
                    let inc = path_to_incr.as_u64().unwrap() + (cmd.arg_increment_value as u64);
                    json!(inc)
                } else {
                    Value::Null
                }
            } else {
                Value::Null
            };

            if new_value.is_null() {
                return print_err("ERR value is not a number");
            }
            return match json.dot_set(&cmd.arg_path, new_value.clone()) {
                Ok(_) => {
                    increment_mutation_counter();
                    print_integer(new_value.as_i64().unwrap())
                }
                Err(e) => {
                    print_err("ERR value not set")
                }
            };
        }
    };
}

pub fn jincr_by_float(cmd: &JIncrByFloatCmd) -> String {
    let mut map: Arc<DashMap<String, Value>> = JSON_BTREE.clone();
    return match map.get_mut(&cmd.arg_key) {
        None => {
            return print_err("ERR key not found");
        }
        Some(mut j) => {
            let mut json = j.value_mut();
            let mut path_to_incr = json.dot_get(&cmd.arg_path).unwrap_or(Some(Value::Null)).unwrap_or(Value::Null);

            if path_to_incr.is_null() {
                let new_value = json!(cmd.arg_increment_value);
                json.dot_set(&cmd.arg_path.to_owned(), new_value.clone());
                increment_mutation_counter();
                return print_str(&new_value.to_string());
            }
            let new_value = if path_to_incr.is_number() {
                if path_to_incr.is_i64() {
                    return print_err("ERR value is not a float");
                } else if path_to_incr.is_f64() {
                    let inc = path_to_incr.as_f64().unwrap() + (cmd.arg_increment_value);
                    json!(inc)
                } else if path_to_incr.is_u64() {
                    return print_err("ERR value is not a float");
                } else {
                    Value::Null
                }
            } else {
                Value::Null
            };

            if new_value.is_null() {
                return print_err("ERR value is not a number");
            }
            return match json.dot_set(&cmd.arg_path, new_value.clone()) {
                Ok(_) => {
                    increment_mutation_counter();
                    print_str(&new_value.to_string())
                }
                Err(e) => {
                    print_err("ERR value not set")
                }
            };
        }
    };
}


