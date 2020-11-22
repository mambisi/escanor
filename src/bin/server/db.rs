use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::collections::HashMap;

extern crate regex;

use sled;
use crate::error::DatabaseError;
use crate::{config, unit_conv};
use crate::network::Context;
use crate::command::SetCmd;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use crate::geo::{GeoPoint2D, Circle};
use crate::command::*;
use crate::printer::*;
use crate::error::ParseDataError;
use crate::util;
use rstar::{RTree, Point};
use crate::util::Location;

extern crate nanoid;
extern crate glob;

use nanoid::nanoid;
use lazy_static;
use nom::lib::std::str::FromStr;
use futures::core_reexport::num::ParseIntError;
use bincode::ErrorKind;
use sled::{MergeOperator, Error, IVec};
use std::collections::BTreeSet;
use nom::lib::std::collections::HashSet;
use crate::unit_conv::{Units};
use cookie_factory::lib::std::fmt::Formatter;

extern crate jsonpath_lib as jsonpath;
extern crate json_dotpath;
use json_dotpath::DotPaths;

// Special Keys
const DATABASE_PATH_PREFIX: &str = "dbs/";
const DEFAULT_DATABASE_PATH: &str = "dbs/db0";
const DEFAULT_DATABASE_NAME: &str = "db0";

lazy_static! {
    static ref DBS : Arc<RwLock<HashMap<String,sled::Db>>> = Arc::new(RwLock::new(HashMap::new()));
}

trait DataTransform {
    fn as_str(&self) -> &str;
    fn as_int(&self) -> i64;
    fn as_float(&self) -> f64;
    fn as_json(&self) -> Value;
}

trait RespResponse {
    fn to_resp(&self) -> String;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeoTree {
    rtree: RTree<GeoPoint2D>,
    btree: HashSet<GeoPoint2D>,
}

impl GeoTree {
    pub fn new() -> Self {
        GeoTree {
            rtree: RTree::new(),
            btree: HashSet::new(),
        }
    }

    pub fn with_items(items: Vec<GeoPoint2D>) -> Self {
        let mut btree = HashSet::new();
        btree.extend(items.iter().map(|i| { i.to_owned() }));
        GeoTree {
            rtree: RTree::bulk_load(items),
            btree,
        }
    }
    pub fn insert(&mut self, p: GeoPoint2D) {
        self.btree.insert(p.clone());
        self.rtree.insert(p);
    }
    pub fn delete(&mut self, tag: &str) -> bool {
        let point = GeoPoint2D::new(tag.to_owned());
        let saved_point = match self.btree.get(&point) {
            None => {
                return  false
            }
            Some(s) => {
                s.to_owned()
            }
        };

        let r = match self.rtree.remove(&saved_point) {
            None => {
                false
            }
            Some(_) => {
                true
            }
        };
        let l = self.btree.remove(&saved_point);
        return r && l;
    }
    pub fn get(&self, tag: &str) -> Option<&GeoPoint2D> {
        let point = GeoPoint2D::new(tag.to_owned());
        self.btree.get(&point)
    }
    pub fn locate_at_point(&self, point: &[f64; 2]) -> Option<&GeoPoint2D> {
        self.rtree.locate_at_point(&point)
    }
    pub fn merge(&mut self, other: &Self) {
        other.rtree.iter().for_each(|point| {
            self.rtree.insert(point.clone())
        });
        self.btree.extend(other.btree.iter().map(|v| v.clone()))
    }
}

impl GeoTree {
    pub fn iter(self: &mut Self) -> impl Iterator<Item=&GeoPoint2D>
    {
        self.btree.iter()
    }
}

impl std::fmt::Display for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", "ERR syntax error")
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Data {
    String(String),
    Int(i64),
    Float(f64),
    Json(Vec<u8>),
    GeoTree(GeoTree),
    Null,
}

impl FromStr for Data {
    type Err = ParseDataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseDataError);
        }

        let first_char = s.chars().nth(0).unwrap();

        if !(first_char.is_numeric() || first_char == '-') {
            return Ok(Data::String(s.to_owned()));
        }

        match s.parse::<i64>() {
            Ok(i) => {
                return Ok(Data::Int(i));
            }
            Err(_) => {}
        };

        match s.parse::<f64>() {
            Ok(i) => {
                return Ok(Data::Float(i));
            }
            Err(_) => {}
        };

        return Ok(Data::String(s.to_owned()));
    }
}

impl Data {
    fn from_vec(vec: &[u8]) -> Result<Self, ParseDataError> {
        return match bincode::deserialize::<Data>(vec) {
            Ok(d) => {
                Ok(d)
            }
            Err(e) => {
                debug!("Parse error: {}",e);
                Err(ParseDataError)
            }
        };
    }
}


impl RespResponse for Data {
    fn to_resp(&self) -> String {
        match self {
            Data::String(d) => {
                print_string(&d)
            }
            Data::Int(d) => {
                print_integer(d)
            }
            Data::Float(d) => {
                print_string(&d.to_string())
            }
            Data::Json(d) => {
                let json_string = serde_json::to_string_pretty(d).unwrap_or("nil".to_string());
                print_string(&json_string)
            }
            Data::GeoTree(_) => {
                print_err("ERR wrong data type")
            }
            Data::Null => {
                print_str("nil")
            }
        }
    }
}


pub async fn init() {
    lazy_static::initialize(&DBS);
    let default_db = sled::open(DEFAULT_DATABASE_PATH).expect("failed to open database");


    fn data_merge(
        _key: &[u8],               // the key being merged
        old_value: Option<&[u8]>,  // the previous value, if one existed
        merged_bytes: &[u8],        // the new bytes being merged in
    ) -> Option<Vec<u8>> {
        let old_data = match old_value {
            None => {
                Data::Null
            }
            Some(bytes) => {
                Data::from_vec(bytes).unwrap()
            }
        };

        let new_data = match Data::from_vec(merged_bytes) {
            Ok(r) => {
                r
            }
            Err(_) => {
                return match old_data {
                    Data::Null => {
                        None
                    }
                    o => {
                        let v = bincode::serialize(&o).unwrap();
                        Some(v)
                    }
                };
            }
        };


        let merge_result = match (old_data, new_data) {
            (Data::String(o), Data::String(n)) => {
                let op = o + &n;
                let v = Data::String(op);
                v
            }

            (Data::Int(o), Data::Int(n)) => {
                Data::Int(n)
            }

            (Data::Float(o), Data::Float(n)) => {
                Data::Float(n)
            }

            (Data::Json(mut o), Data::Json(n)) => {
                let mut a : Value = serde_json::from_slice(&o).unwrap();
                let b: Value = serde_json::from_slice(&n).unwrap_or(Value::Null);
                util::merge(&mut a, &b);
                Data::Json(serde_json::to_vec(&a).unwrap())
            }

            (Data::GeoTree(mut o), Data::GeoTree(n)) => {
                o.merge(&n);
                Data::GeoTree(o)
            }

            (Data::Null, n) => {
                n
            }
            _ => {
                return None;
            }
        };
        let v = bincode::serialize(&merge_result).unwrap();
        Some(v)
    }
    default_db.set_merge_operator(data_merge);
    let mut dbs_writer = DBS.write().unwrap();
    dbs_writer.insert("db0".to_string(), default_db);
}

fn fetch_data(db: &Option<String>, key: &str) -> Result<Data, String> {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, db);
    let k = key.as_bytes();
    return match db.get(k) {
        Ok(r) => {
            match r {
                None => {
                    Err(print_err("KEY_NOT_FOUND"))
                }
                Some(vec) => {
                    match Data::from_vec(&vec) {
                        Ok(d) => {
                            Ok(d)
                        }
                        Err(_) => {
                            Err(print_err("CORRUPT_DATA"))
                        }
                    }
                }
            }
        }
        Err(_) => {
            Err(print_err("KEY_NOT_FOUND"))
        }
    };
}

pub fn auth(context: &mut Context, cmd: &AuthCmd) -> String {
    context.client_auth_key = Some(cmd.arg_password.to_owned());
    if !context.auth_is_required {
        return print_ok();
    }

    let auth_key = match &context.auth_key {
        Some(k) => k.to_owned(),
        None => {
            return print_err("ERR internal error");
        }
    };

    let client_auth_key = match &context.client_auth_key {
        Some(k) => k.to_owned(),
        None => {
            return print_err("ERR internal error");
        }
    };

    if auth_key == client_auth_key {
        context.client_authenticated = true
    } else {
        context.client_authenticated = false
    }
    return if context.client_authenticated {
        print_ok()
    } else {
        print_err("ERR auth failed")
    };
}

pub fn select() {}

fn fetch_db<'a>(dbs: &'a RwLockReadGuard<HashMap<String, sled::Db>>, name: &'a Option<String>) -> &'a sled::Db {
    match name {
        None => {
            dbs.get(DEFAULT_DATABASE_NAME).unwrap()
        }
        Some(s) => {
            dbs.get(s).unwrap()
        }
    }
}

pub fn set(context: &Context, cmd: &SetCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let v = bincode::serialize(&cmd.arg_value).unwrap();
    let k = cmd.arg_key.as_bytes();
    db.insert(k, v);
    print_ok()
}

pub fn get_set(context: &Context, cmd: &GetSetCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();
    let v = bincode::serialize(&cmd.arg_value).unwrap();

    match db.insert(k, v) {
        Ok(r) => {
            let old_raw_data = match r {
                None => {
                    return print_str("nil");
                }
                Some(old) => {
                    old
                }
            };
            let data: Data = bincode::deserialize(old_raw_data.to_vec().as_slice()).unwrap();
            data.to_resp()
        }
        Err(_) => {
            print_err("ERR")
        }
    }
}

pub fn random_key(context: &Context, cmd: &RandomKeyCmd) -> String {
    let key = nanoid!(25, &util::ALPHA_NUMERIC);
    print_string(&key)
}

pub fn get(context: &Context, cmd: &GetCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();
    return match db.get(k) {
        Ok(r) => {
            let raw_data = match r {
                None => {
                    return print_str("nil");
                }
                Some(r) => {
                    r
                }
            };
            let data: Data = bincode::deserialize(raw_data.to_vec().as_slice()).unwrap();
            data.to_resp()
        }
        Err(_) => {
            print_str("nil")
        }
    };
}

pub fn exists(context: &Context, cmd: &ExistsCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let mut found_count: i64 = 0;
    for key in &cmd.keys {
        let k = key.as_bytes();
        if db.contains_key(k).unwrap_or(false) {
            found_count += 1;
        }
    }
    print_integer(&found_count)
}

pub fn info(context: &Context, _cmd: &InfoCmd) -> String {
    print_err("ERR")
}

pub fn db_size(context: &Context, _cmd: &DBSizeCmd) -> String {
    print_err("ERR")
}

pub fn del(context: &Context, cmd: &DelCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();
    let mut count: i64 = 0;

    match db.remove(k) {
        Ok(_) => {
            count += 1
        }
        Err(_) => {}
    }

    print_integer(&count)
}

pub fn persist(context: &Context, cmd: &PersistCmd) -> String {
    print_err("ERR")
}

pub fn ttl(context: &Context, cmd: &TTLCmd) -> String {
    print_err("ERR")
}

pub fn expire(context: &Context, cmd: &ExpireCmd) -> String {
    print_err("ERR")
}

pub fn expire_at(context: &Context, cmd: &ExpireAtCmd) -> String {
    print_err("ERR")
}

pub fn incr_by(context: &Context, cmd: &IncrByCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();
    let increment = cmd.arg_value;
    let updated_data = db.update_and_fetch(k, |old| -> Option<Vec<u8>> {
        let data = match old {
            None => {
                Data::Int(0)
            }
            Some(bytes) => {
                match Data::from_vec(bytes) {
                    Ok(d) => {
                        d
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
        };

        match data {
            Data::String(_) => {
                None
            }
            Data::Int(d) => {
                let num = d + increment;
                let v = bincode::serialize(&Data::Int(num)).unwrap();
                Some(v)
            }
            Data::Float(d) => {
                let num = d + (increment as f64);
                let v = bincode::serialize(&Data::Float(num)).unwrap();
                Some(v)
            }
            Data::Json(_) => {
                None
            }
            Data::GeoTree(_) => {
                None
            }
            Data::Null => {
                None
            }
        }
    });


    match updated_data {
        Ok(bytes) => {
            let vb = bytes.unwrap();
            match Data::from_vec(&vb) {
                Ok(d) => {
                    return d.to_resp();
                }
                _ => {}
            }
        }
        _ => {}
    }


    print_err("ERR")
}

pub fn keys(context: &Context, cmd: &KeysCmd) -> String {
    let mut prefix = String::new();
    for c in cmd.pattern.chars() {
        match c {
            '*' | '?' | '[' => {
                break;
            }
            c => {
                prefix.push(c);
            }
        }
    }
    let pattern_marcher = match glob::Pattern::new(&cmd.pattern) {
        Ok(p) => p,
        Err(_e) => {
            return print_err("ERR invalid pattern");
        }
    };

    let mut keys: Vec<String> = vec![];
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    for r in db.scan_prefix(prefix) {
        match r {
            Ok((k, v)) => {
                let key = String::from_utf8(k.to_vec()).unwrap();
                if pattern_marcher.matches(&key) {
                    keys.push(key)
                }
            }
            Err(_) => {}
        };
    };
    print_arr(keys)
}

pub fn geo_add(context: &Context, cmd: &GeoAddCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();

    //db.fetch_and_update()

    let mut geo_tree = GeoTree::new();

    let items_count = cmd.items.len() as i64;

    cmd.items.iter().for_each(|(lat, lng, tag)| {
        let tag = tag.to_owned();
        let lat = lat.to_owned();
        let lng = lng.to_owned();
        let point = GeoPoint2D::with_cord(tag, lat, lng);
        geo_tree.insert(point);
    });

    let v = bincode::serialize(&Data::GeoTree(geo_tree)).unwrap();
    db.merge(k, v);
    print_integer(&items_count)
}

pub fn geo_hash(context: &Context, cmd: &GeoHashCmd) -> String {
    let data = match fetch_data(&context.db, &cmd.arg_key) {
        Ok(d) => {
            d
        }
        Err(e) => {
            return e;
        }
    };

    let geo_tree = match data {
        Data::GeoTree(t) => {
            t
        }
        _ => {
            return print_err("ERR");
        }
    };

    let mut geo_hashes: Vec<&String> = vec![];
    let empty_string = String::new();

    for tag in &cmd.items {
        match geo_tree.get(tag) {
            Some(point) => {
                geo_hashes.push(point.hash());
            }
            None => {
                geo_hashes.push(&empty_string)
            }
        };
    }

    print_string_arr(geo_hashes)
}

pub fn geo_dist(context: &Context, cmd: &GeoDistCmd) -> String {
    let data = match fetch_data(&context.db, &cmd.arg_key) {
        Ok(d) => {
            d
        }
        Err(e) => {
            return e;
        }
    };

    let geo_tree = match data {
        Data::GeoTree(t) => {
            t
        }
        _ => {
            return print_err("ERR");
        }
    };

    let member_1 = match geo_tree.get(&cmd.arg_mem_1) {
        None => {
            return print_err("ERR member 1 not found");
        }
        Some(p) => {
            p
        }
    };

    let member_2 = match geo_tree.get(&cmd.arg_mem_2) {
        None => {
            return print_err("ERR member 2 not found");
        }
        Some(p) => {
            p
        }
    };

    let distance = util::haversine_distance(Location { latitude: member_1.x_cord(), longitude: member_1.y_cord() },
                                            Location { latitude: member_2.x_cord(), longitude: member_2.y_cord() },
                                            cmd.arg_unit.clone());
    print_string(&distance.to_string())
}

pub fn geo_radius(context: &Context, cmd: &GeoRadiusCmd) -> String {
    let data = match fetch_data(&context.db, &cmd.arg_key) {
        Ok(d) => {
            d
        }
        Err(e) => {
            return e;
        }
    };

    let geo_tree = match data {
        Data::GeoTree(t) => {
            t
        }
        _ => {
            return print_err("ERR Invalid key for data type");
        }
    };

    let radius = match cmd.arg_unit {
        Units::Kilometers => unit_conv::km_m(cmd.arg_radius),
        Units::Miles => unit_conv::mi_m(cmd.arg_radius),
        Units::Meters => cmd.arg_radius,
    };

    let circle = Circle {
        origin: [cmd.arg_lat, cmd.arg_lng],
        radius,
    };

    let nearest_in_radius_array = &mut geo_tree.rtree.nearest_neighbor_iter_with_distance_2(&circle.origin);

    let mut item_string_arr: Vec<Vec<String>> = vec![];

    while let Some((point, dist)) = nearest_in_radius_array.next() {
        if dist <= circle.radius {
            let dist = match cmd.arg_unit {
                Units::Kilometers => unit_conv::m_km(dist),
                Units::Miles => unit_conv::m_mi(dist),
                Units::Meters => dist,
            };

            let string_arr: Vec<String> = vec![point.tag.to_owned(), point.hash().to_owned(), dist.to_string()];
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

pub fn geo_radius_by_member(context: &Context, cmd: &GeoRadiusByMemberCmd) -> String {
    let data = match fetch_data(&context.db, &cmd.arg_key) {
        Ok(d) => {
            d
        }
        Err(e) => {
            return e;
        }
    };

    let geo_tree = match data {
        Data::GeoTree(t) => {
            t
        }
        _ => {
            return print_err("ERR Invalid key for data type");
        }
    };

    let member = match geo_tree.get(&cmd.member) {
        Some(t) => {
            t
        }
        None => {
            return print_err("ERR member 1 not found");
        }
    };

    let cmd = GeoRadiusCmd {
        arg_key: cmd.arg_key.to_owned(),
        arg_lng: member.y_cord(),
        arg_lat: member.x_cord(),
        arg_radius: cmd.arg_radius,
        arg_unit: cmd.arg_unit,
        arg_order: cmd.arg_order,
    };

    geo_radius(&context, &cmd)
}


pub fn geo_pos(context: &Context, cmd: &GeoPosCmd) -> String {
    let data = match fetch_data(&context.db, &cmd.arg_key) {
        Ok(d) => {
            d
        }
        Err(e) => {
            return e;
        }
    };

    let geo_tree = match data {
        Data::GeoTree(t) => {
            t
        }
        _ => {
            return print_err("ERR Invalid key for data type");
        }
    };

    let mut points_array: Vec<Vec<String>> = vec![];

    for s in &cmd.items {
        match geo_tree.get(s) {
            Some(t) => {
                let point_array: Vec<String> = vec![t.x_cord().to_string(), t.y_cord().to_string()];
                points_array.push(point_array)
            }
            None => {
                points_array.push(vec![])
            }
        };
    }

    print_nested_arr(points_array)
}

pub fn geo_del(context: &Context, cmd: &GeoDelCmd) -> String {
    let data = match fetch_data(&context.db, &cmd.arg_key) {
        Ok(d) => {
            d
        }
        Err(e) => {
            return e;
        }
    };

    return match data {
        Data::GeoTree(t) => {
            let dbs = &DBS.read().unwrap();
            let db = fetch_db(dbs, &context.db);
            let k = cmd.arg_key.as_bytes();

            let mut rem_keys_count = 0;

            match db.remove(k) {
                Ok(k) => {
                    rem_keys_count += 1;
                }
                Err(_) => {}
            };

            print_integer(&rem_keys_count)
        }
        _ => {
            print_err("ERR Invalid key for data type")
        }
    };
}

pub fn geo_remove(context: &Context, cmd: &GeoRemoveCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();
    let mut rm_count : i64 = 0;
    db.update_and_fetch(k, |old| -> Option<Vec<u8>> {
        let data = match old {
            None => {
                Data::Null
            }
            Some(bytes) => {
                match Data::from_vec(bytes) {
                    Ok(d) => {
                        d
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
        };

        match data {
            Data::GeoTree(mut t) => {
                for s in &cmd.items {
                    if t.delete(s) {
                        rm_count += 1;
                    }
                }

                let v = bincode::serialize(&Data::GeoTree(t)).unwrap();
                Some(v)
            }
            _ => {
                None
            }
        }
    });

    print_integer(&rm_count)
}

pub fn geo_json(context: &Context, cmd: &GeoJsonCmd) -> String {
    let data = match fetch_data(&context.db, &cmd.arg_key) {
        Ok(d) => {
            d
        }
        Err(e) => {
            return e;
        }
    };

    let mut geo_arr: Vec<GeoPoint2D> = vec![];

    match data {
        Data::GeoTree(t) => {
            for s in &cmd.items {
                match t.get(s) {
                    Some(t) => {
                        geo_arr.push(t.to_owned())
                    }
                    None => {}
                };
            }
        }
        _ => {
            return print_err("ERR Invalid key for data type")
        }
    }
    print_string(&build_geo_json(&geo_arr).to_string())
}

// JSET, JGET, JDEL, JPATH, JMERGE
pub fn jset_raw(context: &Context, cmd: &JSetRawCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();

    let json_value: Value = match serde_json::from_str(&cmd.arg_value) {
        Ok(t) => t,
        Err(_) => { return print_err("ERR invalid json"); }
    };


    let json_b = serde_json::to_vec(&json_value).unwrap();

    let v = bincode::serialize(&Data::Json(json_b)).unwrap();
    db.insert(k,v);
    print_ok()
}

pub fn jset(context: &Context, cmd: &JSetCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();
    db.fetch_and_update(k, |old| -> Option<Vec<u8>> {
        let data = match old {
            None => {
                let json_b = serde_json::to_vec(&Value::Null).unwrap();
                Data::Json(json_b)
            }
            Some(bytes) => {
                match Data::from_vec(bytes) {
                    Ok(d) => {
                        d
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
        };

        match data {
            Data::Json(mut json_b) => {


                let mut json : Value = serde_json::from_slice(&json_b).unwrap_or(Value::Null);
                let mut ers: Vec<json_dotpath::Error> = vec![];
                for (path, value) in &cmd.arg_set_items {
                    match json.dot_set(path, value.to_owned()) {
                        Ok(_t) => {}
                        Err(e) => {
                            ers.push(e)
                        }
                    };
                }
                if !ers.is_empty() {
                    return None;
                }
                let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();
                Some(v)
            }
            _ => {
                None
            }
        }
    });
    print_ok()
}

pub fn jmerge(context: &Context, cmd: &JMergeCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();
    let mut json: Value = match serde_json::from_str(&cmd.arg_value) {
        Ok(t) => t,
        Err(_) => { return print_err("ERR invalid json"); }
    };
    let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();
    db.merge(k,v);
    print_ok()
}

pub fn jget(context: &Context, cmd: &JGetCmd) -> String {
    let data = match fetch_data(&context.db, &cmd.arg_key) {
        Ok(d) => {
            d
        }
        Err(e) => {
            return e;
        }
    };
    let value = match data {
        Data::Json(json) => {
            let json : Value = serde_json::from_slice(&json).unwrap_or(Value::Null);
            json
        }
        _ => {
            return print_err("ERR Invalid key for data type")
        }
    };

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

pub fn jpath(context: &Context, cmd: &JPathCmd) -> String {
    let data = match fetch_data(&context.db, &cmd.arg_key) {
        Ok(d) => {
            d
        }
        Err(e) => {
            return e;
        }
    };
    let value = match data {
        Data::Json(b) => {
            let json : Value = serde_json::from_slice(&b).unwrap_or(Value::Null);
            json
        }
        _ => {
            return print_err("ERR Invalid key for data type")
        }
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
    print_arr(j_strings)
}

pub fn jdel(context: &Context, cmd: &JDelCmd) -> String {
    let data = match fetch_data(&context.db, &cmd.arg_key) {
        Ok(d) => {
            d
        }
        Err(e) => {
            return e;
        }
    };

    return match data {
        Data::Json(_) => {
            let dbs = &DBS.read().unwrap();
            let db = fetch_db(dbs, &context.db);
            let k = cmd.arg_key.as_bytes();

            let mut rem_keys_count = 0;

            match db.remove(k) {
                Ok(k) => {
                    rem_keys_count += 1;
                }
                Err(_) => {}
            };

            print_integer(&rem_keys_count)
        }
        _ => {
            print_err("ERR Invalid key for data type")
        }
    };
}

pub fn jrem(context: &Context, cmd: &JRemCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();
    let mut removal_count = 0;
    db.fetch_and_update(k, |old| -> Option<Vec<u8>> {
        let data = match old {
            None => {
                let json_b = serde_json::to_vec(&Value::Null).unwrap();
                Data::Json(json_b)
            }
            Some(bytes) => {
                match Data::from_vec(bytes) {
                    Ok(d) => {
                        d
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
        };

        match data {
            Data::Json(mut json_b) => {

                let mut json : Value = serde_json::from_slice(&json_b).unwrap_or(Value::Null);
                &cmd.arg_paths.iter().for_each(|s| {
                    match json.dot_remove(s) {
                        Ok(_) => {
                            removal_count += 1;
                        }
                        Err(_) => {}
                    };
                });
                let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();
                Some(v)
            }
            _ => {
                None
            }
        }
    });
    print_integer(&removal_count)
}


pub fn jincr_by(context: &Context, cmd: &JIncrByCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();
    let mut _value: i64 = 0;
    let data = db.fetch_and_update(k, |old| -> Option<Vec<u8>> {
        let data = match old {
            None => {
                let json_b = serde_json::to_vec(&Value::Null).unwrap();
                Data::Json(json_b)
            }
            Some(bytes) => {
                match Data::from_vec(bytes) {
                    Ok(d) => {
                        d
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
        };

        match data {
            Data::Json(mut json_b) => {

                let mut json : Value = serde_json::from_slice(&json_b).unwrap_or(Value::Null);
                let path_to_incr = json.dot_get(&cmd.arg_path).unwrap_or(Some(Value::Null)).unwrap_or(Value::Null);

                if path_to_incr.is_null() {
                    let new_value = json!(cmd.arg_increment_value);
                    json.dot_set(&cmd.arg_path.to_owned(), new_value.clone());
                    let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();
                    _value = new_value.as_i64().unwrap();
                    return Some(v)
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
                    let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();
                    return Some(v)
                }
                json.dot_set(&cmd.arg_path, new_value.clone());
                let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();
                _value = new_value.as_i64().unwrap();
                Some(v)
            }
            _ => {
                None
            }
        }
    });

    print_integer(&_value)
}

pub fn jincr_by_float(context: &Context, cmd: &JIncrByFloatCmd) -> String {
    let dbs = &DBS.read().unwrap();
    let db = fetch_db(dbs, &context.db);
    let k = cmd.arg_key.as_bytes();
    let mut _value: f64 = 0.0;
    let data = db.fetch_and_update(k, |old| -> Option<Vec<u8>> {
        let data = match old {
            None => {
                let json_b = serde_json::to_vec(&Value::Null).unwrap();
                Data::Json(json_b)
            }
            Some(bytes) => {
                match Data::from_vec(bytes) {
                    Ok(d) => {
                        d
                    }
                    Err(_) => {
                        return None;
                    }
                }
            }
        };

        match data {
            Data::Json(mut json_b) => {

                let mut json : Value = serde_json::from_slice(&json_b).unwrap_or(Value::Null);
                let path_to_incr = json.dot_get(&cmd.arg_path).unwrap_or(Some(Value::Null)).unwrap_or(Value::Null);

                if path_to_incr.is_null() {
                    let new_value = json!(cmd.arg_increment_value);
                    json.dot_set(&cmd.arg_path.to_owned(), new_value.clone());
                    let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();
                    _value = new_value.as_f64().unwrap();
                    return Some(v)
                }
                let new_value = if path_to_incr.is_number() {
                    if path_to_incr.is_i64() {
                        let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();
                        return Some(v)
                    } else if path_to_incr.is_f64() {
                        let inc = path_to_incr.as_f64().unwrap() + (cmd.arg_increment_value as f64);
                        json!(inc)
                    } else if path_to_incr.is_u64() {
                        let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();

                        return Some(v)
                    } else {
                        Value::Null
                    }
                } else {
                    Value::Null
                };

                if new_value.is_null() {
                    let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();
                    return Some(v)
                }
                json.dot_set(&cmd.arg_path, new_value.clone());
                let v = bincode::serialize(&Data::Json(serde_json::to_vec(&json).unwrap())).unwrap();
                _value = new_value.as_f64().unwrap();
                Some(v)
            }
            _ => {
                None
            }
        }
    });

    print_string(&_value.to_string())
}
