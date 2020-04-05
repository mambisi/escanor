//external crates
extern crate clap;
extern crate console;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate lazy_static;
//modules
mod network;
mod db;
mod command;
mod error;

#[cfg(test)]
mod tests;


use clap::{App, Arg};

use console::style;
use std::sync::{Arc, RwLock};
use std::collections::BTreeMap;
use crate::db::RecordEntry;

#[tokio::main]
async fn main() {
    env_logger::init();
    let matches = App::new("Escanor")
        .version(format!("{}", style("0.1.alpha").cyan()).as_str())
        .author("Mambisi Zempare")
        .about("Escanor is key value in memory database with disk store developed by ByteQuery Ltd.")
        .arg(Arg::with_name("host")
            .short("h")
            .long("host")
            .help("sets the tcp host for the server")
            .default_value("0.0.0.0")
            .takes_value(true))
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .help("sets the tcp port for the server")
            .default_value("8080")
            .takes_value(true))
        .get_matches();

    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap();

    let addrs = format!("{}:{}",host,port);

   network::start_up(addrs.as_str()).await;
}
