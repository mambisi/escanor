use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::collections::HashMap;

extern crate regex;

use sled::{Db, IVec, Error};
use crate::error::DatabaseError;
use crate::config;
use crate::network::Context;
use crate::command::SetCmd;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use crate::geo::GeoPoint2D;
use crate::command::*;
use crate::printer::*;
use crate::util;
use rstar::RTree;

extern crate nanoid;

use nanoid::nanoid;
use lazy_static;

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
pub enum Data {
    String(String),
    Int(i64),
    Float(f64),
    Json(Value),
    GeoTree(RTree<GeoPoint2D>),
}

impl DataTransform for Data {
    fn as_str(&self) -> &str {
        return match self {
            Data::String(s) => {
                s
            }
            Data::Int(i) => {
                ""
            }
            Data::Float(_) => {
                ""
            }
            Data::Json(_) => {
                ""
            }
            Data::GeoTree(_) => {
                ""
            }
        };
    }

    fn as_int(&self) -> i64 {
        unimplemented!()
    }

    fn as_float(&self) -> f64 {
        unimplemented!()
    }

    fn as_json(&self) -> Value {
        unimplemented!()
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
        }
    }
}

pub async fn init() {
    lazy_static::initialize(&DBS);
    let default_db = sled::open(DEFAULT_DATABASE_PATH).expect("failed to open database");
    let mut dbs_writer = DBS.write().unwrap();
    dbs_writer.insert("db0".to_string(), default_db);
}

/*
fn fetch_db(context: &Context, db: &mut sled::Db){

    let db_name = match &context.db {
        Some(name) => {
            let dbs = DBS.clone();
            let mut dbs_writer = dbs.write().unwrap();
            if !dbs_writer.contains_key(name) {
                let db = sled::open(format!("{}{}", DATABASE_PATH_PREFIX, name)).expect("failed to open database");
                dbs_writer.insert(name.to_string(),db);
            }
            name.to_string()
        }
        None => {x
            "db0".to_string()
        }
    };
    let dbs = DBS.clone();
    let mut dbs_reader = dbs.read().unwrap();
    db = dbs_reader.get_mut(&db_name).unwrap();
}
 */

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


pub fn set(context: &Context, cmd: &SetCmd) -> String {
    let dbs = DBS.read().unwrap();
    let db_name = match &context.db {
        None => {
            DEFAULT_DATABASE_NAME.to_string()
        }
        Some(s) => {
            s.to_string()
        }
    };
    let db = dbs.get(&db_name).unwrap();
    let v = bincode::serialize(&cmd.arg_value).unwrap();
    let k = cmd.arg_key.as_bytes();
    db.insert(k, v);
    // Insert Key and KeyType
    print_ok()
}

pub fn get_set(context: &Context, cmd: &GetSetCmd) -> String {
    let dbs = DBS.read().unwrap();
    let db_name = match &context.db {
        None => {
            DEFAULT_DATABASE_NAME.to_string()
        }
        Some(s) => {
            s.to_string()
        }
    };
    let db = dbs.get(&db_name).unwrap();
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
    //MARK : fetch database
    let dbs = DBS.read().unwrap();
    let db_name = match &context.db {
        None => {
            DEFAULT_DATABASE_NAME.to_string()
        }
        Some(s) => {
            s.to_string()
        }
    };
    let db = dbs.get(&db_name).unwrap();
    //

    let k = cmd.arg_key.as_bytes();
    return match db.get(k) {
        Ok(r) => {
            let raw_data = match r {
                None => {
                    return print_str("nil")
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
    let dbs = DBS.read().unwrap();
    let db_name = match &context.db {
        None => {
            DEFAULT_DATABASE_NAME.to_string()
        }
        Some(s) => {
            s.to_string()
        }
    };
    let db = dbs.get(&db_name).unwrap();
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
    print_err("ERR")
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

pub fn incr_by(context: &Context, cmd: &ExpireCmd) -> String {
    print_err("ERR")
}

pub fn keys(context: &Context, cmd: &KeysCmd) -> String {
    print_err("ERR")
}

pub fn geo_add(context: &Context, cmd: &GeoAddCmd) -> String {
    print_err("ERR")
}

pub fn geo_hash(context: &Context, cmd: &GeoHashCmd) -> String {
    print_err("ERR")
}

pub fn geo_dist(context: &Context, cmd: &GeoDistCmd) -> String {
    print_err("ERR")
}

pub fn geo_radius(context: &Context, cmd: &GeoRadiusCmd) -> String {
    print_err("ERR")
}

pub fn geo_radius_by_member(context: &Context, cmd: &GeoRadiusByMemberCmd) -> String {
    print_err("ERR")
}


pub fn geo_pos(context: &Context, cmd: &GeoPosCmd) -> String {
    print_err("ERR")
}

pub fn geo_del(context: &Context, cmd: &GeoDelCmd) -> String {
    print_err("ERR")
}

pub fn geo_remove(context: &Context, cmd: &GeoRemoveCmd) -> String {
    print_err("ERR")
}

pub fn geo_json(context: &Context, cmd: &GeoJsonCmd) -> String {
    print_err("ERR")
}

// JSET, JGET, JDEL, JPATH, JMERGE
pub fn jset_raw(context: &Context, cmd: &JSetRawCmd) -> String {
    print_err("ERR")
}

pub fn jset(context: &Context, cmd: &JSetCmd) -> String {
    print_err("ERR")
}

pub fn jmerge(context: &Context, cmd: &JMergeCmd) -> String {
    print_err("ERR")
}

pub fn jget(context: &Context, cmd: &JGetCmd) -> String {
    print_err("ERR")
}

pub fn jpath(context: &Context, cmd: &JPathCmd) -> String {
    print_err("ERR")
}

pub fn jdel(context: &Context, cmd: &JDelCmd) -> String {
    print_err("ERR")
}

pub fn jrem(context: &Context, cmd: &JRemCmd) -> String {
    print_err("ERR")
}


pub fn jincr_by(context: &Context, cmd: &JIncrByCmd) -> String {
    print_err("ERR")
}

pub fn jincr_by_float(context: &Context, cmd: &JIncrByFloatCmd) -> String {
    print_err("ERR")
}

