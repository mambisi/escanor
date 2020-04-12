extern crate linefeed;
extern crate escanor;

extern crate clap;

use linefeed::{Interface, Prompter, ReadResult, DefaultTerminal};
use linefeed::chars::escape_sequence;
use linefeed::command::COMMANDS;
use linefeed::complete::{Completer, Completion};
use linefeed::inputrc::parse_text;
use linefeed::terminal::Terminal;
use std::io;

extern crate resp;

pub use redis::{create_client, Client};
pub use resp::{encode_slice, Decoder, Value};

use escanor::common::parser;

mod redis;
mod connection;
mod command;

use clap::{App, Arg};

use std::str::FromStr;
use cookie_factory::lib::std::io::Error;

fn main() {
    let matches = App::new("escanor-cli")
        .version("0.0.1")
        .author("Mambisi Zempare <mambisizempare@gmail.com>")
        .arg(
            Arg::with_name("hostname")
                .short("h")
                .long("hostname")
                .help("Server hostname (default: 127.0.0.1).")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .help("Server port (default: 6379).")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("password")
                .short("a")
                .long("password")
                .help("Password to use when connecting to the server.")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("db")
                .short("n")
                .long("db")
                .help("Database number.")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("command")
                .help("command...")
                .required(false)
                .index(1),
        )
        .get_matches();

    let mut db: u16 = 0;
    let mut port: u16 = 6379;
    let mut password = "";
    let mut hostname = "127.0.0.1";

    if let Some(_db) = matches.value_of("db") {
        db = u16::from_str(_db).expect("Failed to read db");
    }
    if let Some(_port) = matches.value_of("port") {
        port = u16::from_str(_port).expect("Failed to read port");
    }
    if let Some(_password) = matches.value_of("password") {
        password = _password;
    }
    if let Some(_hostname) = matches.value_of("hostname") {
        hostname = _hostname;
    }

    let mut interface = Interface::new("Escanor client 0.0.1")?;
    interface.set_prompt("escanor> ")?;

    let mut client: Client = match create_client(hostname, port, password, db) {
        Ok(cli) => {
            println!("Escanor Server [{}]:{} connected.", hostname, port);
            cli
        }
        Err(err) => {
            println!("Escanor Server [{}]:{} connect failed. {}", hostname, port, err);
            return;
        }
    };

    let _ = run_program(&mut client, &mut interface);
}

fn run_program(client: &mut Client, interface: &mut Interface<DefaultTerminal>) -> io::Result<()> {
    while let ReadResult::Input(line) = interface.read_line()? {
        if !line.trim().is_empty() {
            interface.add_history_unique(line.clone());
        }
        let (cmd, args) = split_first_word(&line);
        let commands = parser::parse_raw_cmd(line.clone().as_bytes());
        let ref_commands: Vec<&str> = commands.iter().map(AsRef::as_ref).collect();
        match client.cmd(&ref_commands) {
            Ok(v) => {
                println!("{}", v.to_beautify_string());
            }
            Err(e) => {
                eprintln!("{:?}", e);
            }
        }
    }

    println!("Goodbye.");

    Ok(())
}

fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim();

    match s.find(|ch: char| ch.is_whitespace()) {
        Some(pos) => (&s[..pos], s[pos..].trim_start()),
        None => (s, "")
    }
}
