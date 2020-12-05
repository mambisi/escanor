//external crates
extern crate clap;
extern crate console;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;

extern crate regex;

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
mod json;
mod storage;
mod persistence;
mod rpc;

use clap::{App, Arg};

use console::style;
use std::env;

const APP_NAME: &str = "Escanor";
const APP_VERSION: &str = "0.1.5";
const APP_AUTHORS: &str = "Mambisi Zempare <mambisizempare@gmail.com>";
const APP_HOMEPAGE: &str = "https://github.com/mambisi/escanor";
const APP_ABOUT: &str = "Escanor is key value in memory database with disk store developed by ByteQuery Ltd.";

extern crate app_dirs2;

use app_dirs2::*;
use async_raft::{Raft, RaftStorage, NodeId};
use crate::codec::{ClientRequest, ServerResponse};
use crate::network::Network;
use crate::storage::Storage;
use std::sync::Arc;
use nom::lib::std::collections::HashSet;
use anyhow::Result;
use tracing_subscriber;
use tracing::{debug, error, info, span, warn, Level};

const APP_INFO: AppInfo = AppInfo { name: "escanor", author: "ByteQuery" };


pub type EscanorRaft = Raft<ClientRequest, ServerResponse, Network, Storage>;

lazy_static!(
    pub static ref RAFT : Arc<EscanorRaft> = {
        storage::init();
        let node_id : NodeId = storage::get_node_id();
        let config = Arc::new(async_raft::Config::build("cls".to_owned()).validate().unwrap());
        let network = Arc::new(Network::new());
        let storage = Arc::new(Storage::new(node_id));
        let raft = Arc::new(EscanorRaft::new(node_id, config, network, storage));
        return raft
    };
);


#[tokio::main]
async fn main() -> Result<()> {


    let mut default_log_flag = "";

    if cfg!(debug_assertions) {
        default_log_flag = "debug";
    } else {
        default_log_flag = "info";
    }
    env::set_var("RUST_LOG", default_log_flag);

    tracing_subscriber::fmt::init();

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
        .arg(Arg::with_name("RESET")
            .long("reset")
            .help("resets the config file")
            .takes_value(false))
        .get_matches();

    let host = "127.0.0.1";
    let port = matches.value_of("PORT").unwrap();

    if matches.is_present("RESET") {
        config::write_default_config_file().await;
    }

    let addrs = &format!("{}:{}", host, port);
    info!("Ready to accept connections");
    lazy_static::initialize(&RAFT);
    let mut members = storage::get_cluster_members();
    RAFT.initialize(members).await?;
    info!("PID: {}", std::process::id());
    config::load_conf(true).await?;
    db::init().await;
    network::start_up( addrs).await?;
    storage::monitor_metrics().await;
    Ok(())
}
