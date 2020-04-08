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

extern crate redis_protocol;
extern crate bytes;

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
mod syntax_analyzer;

use clap::{App, Arg};

use console::style;
use std::env;

#[tokio::main]
async fn main() {

    let mut default_log_flag = "";
    if cfg!(debug_assertions) {
        default_log_flag = "debug";
    } else {
        default_log_flag = "info";
    }

    let matches = App::new("Escanor")
        .version(format!("{}", style("0.1.alpha").cyan()).as_str())
        .author("Mambisi Zempare")
        .about("Escanor is key value in memory database with disk store developed by ByteQuery Ltd.")
        .arg(Arg::with_name("PORT")
            .short("p")
            .long("port")
            .help("sets the tcp port for the server")
            .default_value("6867")
            .takes_value(true))
        .get_matches();

    let host = "127.0.0.1";
    let port = matches.value_of("PORT").unwrap();

    let addrs = &format!("{}:{}", host, port);

    env::set_var("RUST_LOG", default_log_flag);

    env_logger::init();

    network::start_up(addrs).await;
}
