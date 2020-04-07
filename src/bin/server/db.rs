use crate::geo::{Circle, GeoPoint2D};
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};
use std::sync::RwLock;
use rstar::{RTree, RTreeObject};
use std::mem;

use crate::command::{SetCmd, GetCmd, DelCmd, KeysCmd, GeoAddCmd, CmdGeoItem, GeoHashCmd, GeoRadiusCmd};
use std::borrow::Borrow;

use lazy_static::lazy_static;
use bytes::{Bytes, BytesMut};
use crate::printer::{print_err, print_record, print_str, print_ok, print_string_arr, print_from_error, print_string, print_arr, print_nested_arr};
use std::rc::Rc;
use crate::geo;
use crate::printer;
use crate::error::CustomMessageError;
use geohash;
use geohash::Coordinate;
lazy_static! {
    static ref BTREE : Arc<RwLock<BTreeMap<String, ESRecord>>> = Arc::new(RwLock::new(BTreeMap::new()));
    static ref GEO_BTREE : Arc<RwLock<BTreeMap<String, HashSet<GeoPoint2D>>>> = Arc::new(RwLock::new(BTreeMap::new()));
    static ref GEO_RTREE : Arc<RwLock<BTreeMap<String, RTree<GeoPoint2D>>>> = Arc::new(RwLock::new(BTreeMap::new()));
}



#[derive(Clone, Debug)]
pub enum DataType {
    String,
    Integer,
}

#[derive(Clone, Debug)]
pub struct ESRecord {
    pub key: String,
    pub value: String,
    pub data_type: DataType,
}


pub fn set(cmd: &SetCmd) -> String {
    let arc: Arc<RwLock<BTreeMap<String, ESRecord>>> = BTREE.clone();
    let mut map = arc.write().unwrap();

    let record = &ESRecord {
        key: cmd.arg_key.to_owned(),
        value: cmd.arg_value.to_owned(),
        data_type: cmd.arg_type.to_owned(),
    };


    match map.insert(record.key.to_owned(), record.to_owned()) {
        Some(prev_rec) => {
            info!("update key {}", prev_rec.key);
        }
        None => {
            info!("insert key {}", record.key);
        }
    };

    print_ok()
}

pub fn get(cmd: &GetCmd) -> String {
    let arc: Arc<RwLock<BTreeMap<String, ESRecord>>> = BTREE.clone();
    let map = arc.read().unwrap();
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

pub fn del(cmd: &DelCmd) -> String {
    let arc: Arc<RwLock<BTreeMap<String, ESRecord>>> = BTREE.clone();
    let mut map = arc.write().unwrap();
    let key = &cmd.arg_key.to_owned();
    return match map.remove(key) {
        Some(r) => {
            print_ok()
        }
        None => {
            print_err("KEY_NOT_FOUND")
        }
    };
}

pub fn list_keys(cmd: &KeysCmd) -> String {
    let arc: Arc<RwLock<BTreeMap<String, ESRecord>>> = BTREE.clone();
    let map = arc.read().unwrap();

    let mut keys: Vec<&String> = vec![];

    for key in map.keys() {
        keys.push(key)
    }
    print_string_arr(keys)
}

pub fn geo_add(cmd: &GeoAddCmd) -> String {
    let r_arc: Arc<RwLock<BTreeMap<String, RTree<GeoPoint2D>>>> = GEO_RTREE.clone();
    let mut r_map = r_arc.write().unwrap();

    let arc: Arc<RwLock<BTreeMap<String, HashSet<GeoPoint2D>>>> = GEO_BTREE.clone();
    let mut map = arc.write().unwrap();


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
        return printer::print_err(&msg);
    }


    let mut bulk_geo_hash_load: Vec<GeoPoint2D> = vec![];

    point_map.iter().for_each(|p| {
        bulk_geo_hash_load.push(p.clone())
    });

    map.insert(cmd.arg_key.to_owned(), point_map);
    //Only executed on debug
    if cfg!(debug_assertions) {
        map.iter().for_each(|(k, v)| {
            debug!("[{}] -> {:?}", k, v);
        });
    }

    r_map.insert(cmd.arg_key.to_owned(), RTree::bulk_load(bulk_geo_hash_load));

    if cfg!(debug_assertions) {
        r_map.iter().for_each(|(k, v)| {
            debug!("[{}] -> {:?}", k, v);
        });
    }
    print_ok()
}

pub fn geo_hash(cmd: &GeoHashCmd) -> String {
    let arc: Arc<RwLock<BTreeMap<String, HashSet<GeoPoint2D>>>> = GEO_BTREE.clone();
    let map = arc.read().unwrap();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();


    let geo_point_hash_set = match map.get(&cmd.arg_key) {
        Some(m) => m,
        None => {
            return return print_err("KEY_NOT_FOUND");;
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
            None => {}
        };
    }

    print_string_arr(geo_hashes)
}

pub fn geo_radius(cmd: &GeoRadiusCmd) -> String {
    let r_arc: Arc<RwLock<BTreeMap<String, RTree<GeoPoint2D>>>> = GEO_RTREE.clone();
    let r_map = r_arc.read().unwrap();
    //let default_hash: HashSet<GeoPoint2D> = HashSet::new();

    let geo_points_rtree = match r_map.get(&cmd.arg_key) {
        Some(m) => m,
        None => {
            return return print_err("KEY_NOT_FOUND");;
        }
    };

    let circle = Circle {
        origin: [cmd.arg_lat, cmd.arg_lng],
        radius: cmd.arg_radius,
    };
    /*
       ["Palermo","190.4424","st0219xsd21"]
    */

    //geo_points_rtree.nearest_neighbor_iter_with_distance(&circle);

    let nearest_in_radius_array = &mut geo_points_rtree.nearest_neighbor_iter_with_distance(&circle.origin);

    let mut item_string_arr : Vec<Vec <String> > = vec![];

    while let Some((point, dist)) = nearest_in_radius_array.next() {

        if dist <= circle.radius {
            let string_arr : Vec<String> = vec![point.tag.to_owned(),point.hash.to_owned(),dist.to_string()];
            &item_string_arr.push( string_arr);
        }
    }

    print_nested_arr(item_string_arr)
}



