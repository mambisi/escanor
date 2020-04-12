extern crate regex;

use crate::{db, printer, unit_conv};
use crate::error;
use crate::util;
use crate::error::SyntaxError;
use crate::tokenizer;
use crate::syntax_analyzer;
use regex::Regex;
use serde::export::Option::Some;
use crate::unit_conv::Units;
use nom::character::complete::char;

pub fn parse(buf: &[u8]) -> Result<Box<dyn Command>, error::SyntaxError> {
    let empty_string = String::new();
    let first_char = buf[0] as char;
    return match first_char {
        '*' | '$' | '+' => {
            parse_resp(buf)
        }
        _ => {
            parse_cli(buf)
        }
    };

    Err(error::SyntaxError)
}

pub fn parse_cli(cmd: &[u8]) -> Result<Box<dyn Command>, error::SyntaxError> {
    let end_chars = &cmd[(cmd.len() - 2)..];
    let last_2_strings = String::from_utf8(end_chars.to_vec()).unwrap_or("".to_string());
    let tokens: Vec<String> = if last_2_strings == "\r\n" {
        tokenizer::generate_tokens(&cmd[..cmd.len() - 2])
    } else {
        tokenizer::generate_tokens(cmd)
    };

    match syntax_analyzer::analyse_token_stream(tokens) {
        Ok(t) => Ok(t),
        Err(e) => Err(SyntaxError)
    }
}

pub fn parse_resp(buf: &[u8]) -> Result<Box<dyn Command>, error::SyntaxError> {
    let tokens: Vec<String> = tokenizer::generate_tokens_from_resp(buf);
    match syntax_analyzer::analyse_token_stream(tokens) {
        Ok(t) => Ok(t),
        Err(e) => Err(SyntaxError)
    }
}

pub trait Command {
    //fn execute(&self, db: &db::DB);
    fn execute(&self) -> String;
}

// Grammar > set [key] [value] ex [exp]
#[derive(Debug)]
pub struct SetCmd {
    pub arg_key: String,
    pub arg_type: db::DataType,
    pub arg_value: String,
    pub arg_exp: u32,
}

pub type CmdGeoItem = (f64, f64, String);

#[derive(Debug)]
pub struct GeoAddCmd {
    pub arg_key: String,
    pub items: Vec<CmdGeoItem>,
}

#[derive(Debug, Clone, Copy)]
pub enum ArgOrder {
    ASC,
    DESC,
    UNSPECIFIED,
}

#[derive(Debug)]
pub struct GeoRadiusCmd {
    pub arg_key: String,
    pub arg_lng: f64,
    pub arg_lat: f64,
    pub arg_radius: f64,
    pub arg_unit: Units,
    pub arg_order: ArgOrder,
}


#[derive(Debug)]
pub struct GeoHashCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}


#[derive(Debug)]
pub struct GeoPosCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}

#[derive(Debug)]
pub struct GeoRadiusByMemberCmd {
    pub arg_key: String,
    pub member: String,
    pub arg_radius: f64,
    pub arg_unit: Units,
    pub arg_order: ArgOrder,
}


#[derive(Debug)]
pub struct GeoDistCmd {
    pub arg_key: String,
    pub arg_mem_1: String,
    pub arg_mem_2: String,
    pub arg_unit: Units,
}

#[derive(Debug)]
pub struct GeoDelCmd {
    pub arg_key: String,
}

#[derive(Debug)]
pub struct GeoRemoveCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}

#[derive(Debug)]
pub struct GeoJsonCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}

#[derive(Debug)]
pub struct ColCreateCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}

// Grammar > get [key]
#[derive(Debug)]
pub struct GetCmd {
    pub arg_key: String
}

// Grammar > del [key]
#[derive(Debug)]
pub struct DelCmd {
    pub arg_key: String
}

#[derive(Debug)]
pub struct KeysCmd {
    pub pattern: String
}

#[derive(Debug)]
pub struct ExistsCmd {
    pub keys: Vec<String>,
}


#[derive(Debug)]
pub struct InfoCmd;


#[derive(Debug)]
pub struct PingCmd;

#[derive(Debug)]
pub struct JSetCmd {
    pub arg_key: String,
    pub arg_value: String
}

#[derive(Debug)]
pub struct JMergeCmd {
    pub arg_key: String,
    pub arg_value: String
}
#[derive(Debug)]
pub struct JGetCmd {
    pub arg_key: String
}
#[derive(Debug)]
pub struct JPathCmd {
    pub arg_key: String,
    pub arg_selector: String
}
#[derive(Debug)]
pub struct JDelCmd {
    pub arg_key: String
}

impl Command for PingCmd {
    fn execute(&self) -> String {
        printer::print_pong()
    }
}

impl Command for SetCmd {
    fn execute(&self) -> String {
        db::set(self)
    }
}

impl Command for GetCmd {
    fn execute(&self) -> String {
        db::get(self)
    }
}

impl Command for DelCmd {
    fn execute(&self) -> String {
        db::del(self)
    }
}

impl Command for KeysCmd {
    fn execute(&self) -> String {
        db::keys(self)
    }
}

impl Command for ExistsCmd {
    fn execute(&self) -> String {
        db::exists(self)
    }
}

impl Command for InfoCmd {
    fn execute(&self) -> String {
        db::info(self)
    }
}

impl Command for GeoAddCmd {
    fn execute(&self) -> String {
        db::geo_add(self)
    }
}

impl Command for GeoHashCmd {
    fn execute(&self) -> String {
        db::geo_hash(self)
    }
}

impl Command for GeoRadiusCmd {
    fn execute(&self) -> String {
        db::geo_radius(self)
    }
}

impl Command for GeoDistCmd {
    fn execute(&self) -> String {
        db::geo_dist(self)
    }
}

impl Command for GeoRadiusByMemberCmd {
    fn execute(&self) -> String {
        db::geo_radius_by_member(self)
    }
}

impl Command for GeoPosCmd {
    fn execute(&self) -> String {
        db::geo_pos(self)
    }
}

impl Command for GeoDelCmd {
    fn execute(&self) -> String {
        db::geo_del(self)
    }
}

impl Command for GeoRemoveCmd {
    fn execute(&self) -> String {
        db::geo_remove(self)
    }
}

impl Command for GeoJsonCmd {
    fn execute(&self) -> String {
        db::geo_json(self)
    }
}


impl Command for JSetCmd {
    fn execute(&self) -> String {
        db::jset(self)
    }
}

impl Command for JMergeCmd {
    fn execute(&self) -> String {
        db::jmerge(self)
    }
}

impl Command for JGetCmd {
    fn execute(&self) -> String {
        db::jget(self)
    }
}

impl Command for JPathCmd {
    fn execute(&self) -> String {
        db::jpath(self)
    }
}

impl Command for JDelCmd {
    fn execute(&self) -> String {
        db::jdel(self)
    }
}
