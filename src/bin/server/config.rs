use std::collections::HashMap;
use lazy_static::lazy_static;
use std::sync::{RwLock, Arc, RwLockReadGuard, RwLockWriteGuard};
use crate::printer;
lazy_static! {
    static ref CONFIG_HASH_MAP : Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct DatabaseConf {
    pub save_after: usize,
    pub mutations: usize,
}

pub struct NetConf {
    pub port: usize,
    pub bind: String,
    pub max_packet: usize,
    pub max_connections: usize,
}

pub struct Conf {
    pub database: DatabaseConf,
    pub network: NetConf,
}

const MUTABLE_CONF_KEYS: [&str; 2] = ["database.save_after", "database.mutations"];

impl Conf {
    fn new(map: &HashMap<String, String>) -> Conf {
        let default_n_port = String::from("6379");
        let default_n_bind = String::from("127.0.0.1");
        let default_n_packet = String::from("10");
        let default_n_conns = String::from("0");

        let default_d_save_after = String::from("60");
        let default_d_muts = String::from("4");

        let net_conf = NetConf {
            port: map.get("network.port").unwrap_or(&default_n_port).parse::<usize>().unwrap(),
            bind: map.get("network.bind").unwrap_or(&default_n_bind).to_owned(),
            max_packet: map.get("network.max_packet").unwrap_or(&default_n_packet).parse::<usize>().unwrap(),
            max_connections: map.get("network.max_connections").unwrap_or(&default_n_conns).parse::<usize>().unwrap(),
        };

        let db_conf = DatabaseConf {
            save_after: map.get("database.save_after").unwrap_or(&default_d_save_after).parse::<usize>().unwrap(),
            mutations: map.get("database.mutations").unwrap_or(&default_d_muts).parse::<usize>().unwrap(),
        };

        Conf {
            database: db_conf,
            network: net_conf,
        }
    }

    fn from_rw(map: &RwLockReadGuard<HashMap<String, String>>) -> Conf {
        let default_n_port = String::from("6379");
        let default_n_bind = String::from("127.0.0.1");
        let default_n_packet = String::from("10");
        let default_n_conns = String::from("0");

        let default_d_save_after = String::from("60");
        let default_d_muts = String::from("4");

        let net_conf = NetConf {
            port: map.get("network.port").unwrap_or(&default_n_port).parse::<usize>().unwrap(),
            bind: map.get("network.bind").unwrap_or(&default_n_bind).to_owned(),
            max_packet: map.get("network.max_packet").unwrap_or(&default_n_packet).parse::<usize>().unwrap(),
            max_connections: map.get("network.max_connections").unwrap_or(&default_n_conns).parse::<usize>().unwrap(),
        };

        let db_conf = DatabaseConf {
            save_after: map.get("database.save_after").unwrap_or(&default_d_save_after).parse::<usize>().unwrap(),
            mutations: map.get("database.mutations").unwrap_or(&default_d_muts).parse::<usize>().unwrap(),
        };

        Conf {
            database: db_conf,
            network: net_conf,
        }
    }


    fn to_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();

        map.insert("database.save_after".to_owned(), self.database.save_after.to_string());
        map.insert("database.mutations".to_owned(), self.database.mutations.to_string());
        map.insert("network.port".to_owned(), self.database.save_after.to_string());
        map.insert("network.bind".to_owned(), self.database.save_after.to_string());
        map.insert("network.max_packet".to_owned(), self.database.save_after.to_string());
        map.insert("network.max_connections".to_owned(), self.database.save_after.to_string());

        map
    }
}

pub fn conf() -> Conf {
    let config_map: RwLockReadGuard<HashMap<String, String>> = CONFIG_HASH_MAP.read().unwrap();
    Conf::from_rw(&config_map)
}

pub fn get_conf_by_key(key: &String) -> Option<String> {
    let config_map: RwLockReadGuard<HashMap<String, String>> = CONFIG_HASH_MAP.read().unwrap();
    let value = match config_map.get(key) {
        None => { "" }
        Some(v) => { v }
    };
    if value.is_empty() {
        return None;
    }
    return Some(value.to_owned());
}

pub fn set_conf(key: &String, value: &String) -> Result<(), String> {
    let mut matched_a_key = false;
    for s in MUTABLE_CONF_KEYS.to_vec() {
        if key == s {
            matched_a_key = true;
            break;
        }
    };
    if !matched_a_key {
        return Err("invalid key".to_owned());
    }
    let mut config_map: RwLockWriteGuard<HashMap<String, String>> = CONFIG_HASH_MAP.write().unwrap();
    config_map.insert(key.to_owned(), value.to_owned());
    Ok(())
}

pub fn rewrite_conf() -> String {

}