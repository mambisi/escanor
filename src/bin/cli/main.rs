extern crate linefeed;
extern crate escanor;

extern crate clap;



use linefeed::{Command, Function, Interface, Prompter, ReadResult, Terminal, DefaultTerminal, Signal};

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



extern crate serde;
extern crate serde_json;
extern crate colored_json;

use serde_json::{Value as JsonValue};
use colored_json::to_colored_json_auto;


const DEMO_FN_SEQ: &str = "\n^c";

fn main() -> io::Result<()> {
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
    interface.set_report_signal(Signal::Interrupt, true);

    println!("escanor cli 1.0");
    println!("Press Ctrl+C or enter \"exit\" to quit. If not connect press enter to retry connection");
    loop {
        match create_client(hostname, port, password, db) {
            Ok(mut cli) => {
                interface.set_prompt(&format!("{}:{}> ", hostname, port)).unwrap();
                let _ = run_program(&mut cli, &mut interface);
            }
            Err(_err) => {
                interface.set_prompt("not connected> ")?;
                let res = interface.read_line()?;
                match res {
                    ReadResult::Eof => { return Ok(()); }
                    ReadResult::Input(line) => {
                        if !line.trim().is_empty() {
                            interface.add_history_unique(line.clone());
                        }
                        let (cmd, _args) = split_first_word(&line);
                        ex_sys_cmd(cmd, &mut interface);
                        continue;
                    }
                    ReadResult::Signal(sig) => {
                        if sig == Signal::Interrupt {
                            std::process::exit(1);
                        }
                    }
                }
                return Ok(());
            }
        };
    }
}

fn run_program(client: &mut Client, interface: &mut Interface<DefaultTerminal>) -> io::Result<()> {
    loop {
        let res = interface.read_line()?;
        match res {
            ReadResult::Eof => break,
            ReadResult::Input(line) => {
                if !line.trim().is_empty() {
                    interface.add_history_unique(line.clone());
                }
                let (cmd, _args) = split_first_word(&line);
                ex_sys_cmd(cmd,interface);
                let commands = parser::parse_raw_cmd(line.clone().as_bytes());
                let ref_commands: Vec<&str> = commands.iter().map(AsRef::as_ref).collect();
                match client.cmd(&ref_commands) {
                    Ok(v) => {
                        let va = v.to_beautify_string();
                        if cmd.to_uppercase() == "JGET" || cmd.to_uppercase() == "GEOJSON" || cmd.to_uppercase() == "JPATH" {
                            let jva = va.to_owned();
                            let json : JsonValue = serde_json::from_str(&jva.trim_matches(&['"'] as &[_])).unwrap_or(JsonValue::Null);
                            if json.is_null() || json.is_f64() {
                                println!("{}", va);
                            }else {
                                println!("{}",to_colored_json_auto(&json).unwrap_or(va.to_owned()))
                            }
                        }else {
                            println!("{}", va.to_owned());
                        }
                    }
                    Err(_e) => {
                        break;
                    }
                }
            }
            ReadResult::Signal(sig) => {
                if sig == Signal::Interrupt {
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}

fn ex_sys_cmd(command: &str, _rinterface: &mut Interface<DefaultTerminal>) {
    match command {
        "exit" => {
            std::process::exit(1);
        }
        _ => {}
    }
}

fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim();

    match s.find(|ch: char| ch.is_whitespace()) {
        Some(pos) => (&s[..pos], s[pos..].trim_start()),
        None => (s, "")
    }
}