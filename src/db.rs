use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::mem;

use crate::command::{SetCmd, GetCmd};
use std::borrow::Borrow;

use lazy_static::lazy_static;

lazy_static! {
    static ref BTREE : Arc<RwLock<BTreeMap<String, RecordEntry>>> = Arc::new(RwLock::new(BTreeMap::new()));
}

#[derive(Clone, Debug)]
pub enum DataType {
    String,
    Number,
    Point,
    Json,
}
#[derive(Clone, Debug)]
pub struct RecordEntry {
    key: String,
    value: String,
    data_type: DataType,
}

pub fn set(cmd : &SetCmd) -> String{
    let arc : Arc<RwLock<BTreeMap<String, RecordEntry>>> = BTREE.clone();
    let mut map = arc.write().unwrap();


    return match  map.insert(cmd.arg_key.to_owned(), RecordEntry {
        key: cmd.arg_key.to_owned(),
        value: cmd.arg_value.to_owned(),
        data_type: DataType::String
    }) {
        Some(t) => {
            println!("set: Ok, count: {} size {} bytes", map.len(), mem::size_of_val(&t));
            "OK".to_owned()
        },
        None => "(error) Internal error".to_owned()
    };
}

pub fn get(cmd : &GetCmd) -> String {
    let arc : Arc<RwLock<BTreeMap<String, RecordEntry>>> = BTREE.clone();
    let map = arc.read().unwrap();
    let key = &cmd.arg_key.to_owned();
    return match map.get(key) {
        Some(r) => {
            r.clone().value
        },
        None => {
            "(error) Not Value found".to_string()
        }
    };
}

pub fn del(cmd : &GetCmd) -> String {
    let arc : Arc<RwLock<BTreeMap<String, RecordEntry>>> = BTREE.clone();
    let mut map = arc.write().unwrap();
    let key = &cmd.arg_key.to_owned();
    return match map.remove(key) {
        Some(r) => {
            r.clone().value
        },
        None => {
            "(error) Not Value found".to_string()
        }
    };
}