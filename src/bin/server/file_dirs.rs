use std::path::{PathBuf};
use app_dirs2::*;
use crate::APP_INFO;


pub fn config_file_path() -> Option<PathBuf> {

    if cfg!(target_os = "linux") {
        let mut  directory  = PathBuf::from("/usr/.conf/escanor");
        if !directory.exists() {
            std::fs::create_dir_all(directory.clone());
        }
        directory.push("config");
        directory.set_extension("yaml");
        return Some(directory);
    }

    let p = match create_file_path(AppDataType::UserConfig, "config", "yaml") {
        None => { return None; }
        Some(p) => { p }
    };
    Some(p)
}

pub fn db_file_path() -> Option<PathBuf> {

    if cfg!(target_os = "linux") {
        let mut  directory  = PathBuf::from("/usr/lib/escanor");
        if !directory.exists() {
            std::fs::create_dir_all(directory.clone());
        }
        directory.push("dump");
        directory.set_extension("esdb");
        return Some(directory);
    }

    let p = match create_file_path(AppDataType::UserCache, "dump", "esdb") {
        None => { return None; }
        Some(p) => { p }
    };
    Some(p)
}

fn create_file_path(datatype: AppDataType, filename: &str, ext: &str) -> Option<PathBuf> {
    let mut path = match app_dir(datatype, &APP_INFO, "") {
        Ok(d) => { d }
        Err(e) => {
            println!("{}", e);
            return None;
        }
    };
    path.push(filename);
    path.set_extension(ext);
    Some(path)
}
