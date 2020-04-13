use std::collections::HashMap;
use lazy_static::lazy_static;
use std::sync::{RwLock, Arc, RwLockReadGuard, RwLockWriteGuard};
use crate::{printer, APP_INFO};
use serde::{Serialize, Deserialize};

lazy_static! {
    static ref CONFIG_HASH_MAP : Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));
}



#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DatabaseConf {
    pub save_after: usize,
    pub mutations: usize,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct NetConf {
    pub port: usize,
    pub bind: String,
    pub max_packet: usize,
    pub max_connections: usize,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Conf {
    pub database: DatabaseConf,
    pub network: NetConf,
}

use tokio::io::{AsyncReadExt,AsyncWriteExt};
use tokio::fs::{OpenOptions, File};
use std::path::{PathBuf, Path};
use serde_yaml;
use crate::file_dirs;
use tokio::runtime::Runtime;
pub async fn load_conf(force_rewrite: bool) -> Result<(),String> {
    let path = match file_dirs::config_file_path() {
        None => {
            info!("Path");
            return Ok(());
        }
        Some(p) => { p }
    };

    if !path.exists() && force_rewrite {
        match rewrite().await {
            Ok(_) => {}
            Err(e) => { return Err(e); }
        };
        load_conf(false);
    }else if !path.exists() && !force_rewrite {
        panic!("Config file not found");
        return Err("Config file not found".to_string())
    }
    //path.join("");
    //rewrite()
    let mut file: File = match OpenOptions::new().read(true).open(path).await {
        Err(e) => {
            panic!("Configuration file not loaded");
            return Err("Configuration file not loaded".to_owned());
        }
        Ok(file) => file,
    };

    let mut contents: Vec<u8> = vec![];
    let n: usize = match file.read_to_end(&mut contents).await {
        Ok(n) => {
            n
        }
        Err(e) => {
            return Err("Error".to_owned());
        }
    };
    let conf: Conf = serde_yaml::from_slice(&contents).unwrap();
    let conf_map = conf.to_map();

    let mut config_map: RwLockWriteGuard<HashMap<String, String>> = CONFIG_HASH_MAP.write().unwrap();

    conf_map.iter().for_each(|(k, v)| {
        config_map.insert(k.to_owned(), v.to_owned());
    });
    info!("Configuration loaded");
    Ok(())
}

pub async fn rewrite() -> Result<(), String> {
    let path = match file_dirs::config_file_path() {
        None => { return Err("Error reading file path".to_owned()); }
        Some(p) => { p }
    };
    let mut file = match OpenOptions::new().write(true).create(true).open(path).await {
        Err(e) => {
            println!("{}", e);
            return Err("Creating reading file path".to_owned());
        }
        Ok(file) => file,
    };

    let conf = conf();
    let buf = serde_yaml::to_vec(&conf).unwrap();
    return match file.write_all(&buf).await {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            Err("".to_owned())
        }
    };
}

const MUTABLE_CONF_KEYS: [&str; 2] = ["database.save_after", "database.mutations"];

impl Conf {
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

const conf

