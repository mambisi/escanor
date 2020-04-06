use crate::geo::{Circle, GeoPoint2D};
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};
use std::sync::RwLock;
use std::mem;

use crate::command::{SetCmd, GetCmd, DelCmd, KeysCmd, GeoAddCmd, CmdGeoItem};
use std::borrow::Borrow;

use lazy_static::lazy_static;
use bytes::{Bytes, BytesMut};
use crate::printer::{print_err, print_record, print_str, print_ok, print_string_arr};
use std::rc::Rc;
use crate::geo;

lazy_static! {
    static ref BTREE : Arc<RwLock<BTreeMap<String, ESRecord>>> = Arc::new(RwLock::new(BTreeMap::new()));
    static ref GEO_BTREE : Arc<RwLock<BTreeMap<String, HashSet<GeoPoint2D>>>> = Arc::new(RwLock::new(BTreeMap::new()));
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
    let arc: Arc<RwLock<BTreeMap<String, HashSet<GeoPoint2D>>>> = GEO_BTREE.clone();
    let mut map = arc.write().unwrap();



    if map.contains_key(&cmd.arg_key) {
        //update previous insertion
        let point_map = map.get_mut(&cmd.arg_key).unwrap();

        cmd.items.iter().for_each(|(lat,lng,tag)| {
            let tag = tag.to_owned();
            let lat = lat.to_owned();
            let lng = lng.to_owned();
            let point = GeoPoint2D {
                tag,
                lat,
                lng,
            };
            point_map.insert(point);
        });

        return print_ok();
    }

    let mut point_map: HashSet<GeoPoint2D> = HashSet::new();

    cmd.items.iter().for_each(|(lat,lng,tag)| {
        let tag = tag.to_owned();
        let lat = lat.to_owned();
        let lng = lng.to_owned();
        let point = GeoPoint2D {
            tag,
            lat,
            lng,
        };
        point_map.insert(point);
    });


    map.insert(cmd.arg_key.to_owned(),point_map);
    info!("Geo BTree len: {}",map.len());
    print_ok()
}


