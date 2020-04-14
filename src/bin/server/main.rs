//external crates
extern crate clap;
extern crate console;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;


//modules
mod network;
mod db;
mod command;
mod error;
mod printer;
mod util;
mod geo;
mod unit_conv;
mod tokenizer;
mod config;
mod syntax_analyzer;
mod file_dirs;
mod codec;

use clap::{App, Arg};

use console::style;
use std::env;

const APP_NAME: &str = "Escanor";
const APP_VERSION: &str = "0.1.0";
const APP_AUTHORS: &str = "Mambisi Zempare <mambisizempare@gmail.com>";
const APP_HOMEPAGE: &str = "https://github.com/mambisi/escanor";
const APP_ABOUT: &str = "Escanor is key value in memory database with disk store developed by ByteQuery Ltd.";

extern crate app_dirs2;
use app_dirs2::*;

const APP_INFO: AppInfo = AppInfo{name: "escanor", author: "ByteQuery"};

#[tokio::main]
async fn main() {
    let mut default_log_flag = "";

    if cfg!(debug_assertions) {
        default_log_flag = "debug";
    } else {
        default_log_flag = "info";
    }


    let matches = App::new(APP_NAME)
        .version(format!("{}", style(APP_VERSION).cyan()).as_str())
        .author(APP_AUTHORS)
        .about(APP_ABOUT)
        .arg(Arg::with_name("PORT")
            .short("p")
            .long("port")
            .help("sets the tcp port for the server")
            .default_value("6379")
            .takes_value(true))
        .get_matches();

    let host = "127.0.0.1";
    let port = matches.value_of("PORT").unwrap();

    let addrs = &format!("{}:{}", host, port);

    env::set_var("RUST_LOG", default_log_flag);
    env_logger::init();

    info!("PID: {}", std::process::id());
    config::load_conf(true).await;
    db::init_db().await;
    network::start_up(addrs).await;
    //network::start_up_ws(addrs)
}
