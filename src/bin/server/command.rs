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
use redis_protocol::types::Frame;
use serde_json::Value;

pub fn compile_frame (frame : Frame) -> Result<Box<dyn Command>, error::SyntaxError> {
    let tokens: Vec<String> = tokenizer::generate_token_from_frame(frame);
    match syntax_analyzer::analyse_token_stream(tokens) {
        Ok(t) => Ok(t),
        Err(e) => Err(SyntaxError)
    }
}

pub fn compile(buf: &[u8]) -> Result<Box<dyn Command>, error::SyntaxError> {
    let empty_string = String::new();
    let first_char = buf[0] as char;
    return match first_char {
        '*' | '$' | '+' => {
            compile_resp(buf)
        }
        _ => {
            compile_raw(buf)
        }
    };

    Err(error::SyntaxError)
}

pub fn compile_raw(cmd: &[u8]) -> Result<Box<dyn Command>, error::SyntaxError> {
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

pub fn compile_resp(buf: &[u8]) -> Result<Box<dyn Command>, error::SyntaxError> {
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

// Key value Commands
#[derive(Debug)]
pub struct SetCmd {
    pub arg_key: String,
    pub arg_type: db::DataType,
    pub arg_value: String,
    pub arg_exp: u32,
}
impl Command for SetCmd {
    fn execute(&self) -> String {
        db::set(self)
    }
}

#[derive(Debug)]
pub struct GetCmd {
    pub arg_key: String
}
impl Command for GetCmd {
    fn execute(&self) -> String {
        db::get(self)
    }
}

#[derive(Debug)]
pub struct DelCmd {
    pub arg_key: String
}
impl Command for DelCmd {
    fn execute(&self) -> String {
        db::del(self)
    }
}

#[derive(Debug)]
pub struct KeysCmd {
    pub pattern: String
}
impl Command for KeysCmd {
    fn execute(&self) -> String {
        db::keys(self)
    }
}

#[derive(Debug)]
pub struct ExistsCmd {
    pub keys: Vec<String>,
}
impl Command for ExistsCmd {
    fn execute(&self) -> String {
        db::exists(self)
    }
}
//

// Geo Spatial Commands
pub type CmdGeoItem = (f64, f64, String);
#[derive(Debug)]
pub struct GeoAddCmd {
    pub arg_key: String,
    pub items: Vec<CmdGeoItem>,
}
impl Command for GeoAddCmd {
    fn execute(&self) -> String {
        db::geo_add(self)
    }
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
impl Command for GeoRadiusCmd {
    fn execute(&self) -> String {
        db::geo_radius(self)
    }
}

#[derive(Debug)]
pub struct GeoHashCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}

impl Command for GeoHashCmd {
    fn execute(&self) -> String {
        db::geo_hash(self)
    }
}


#[derive(Debug)]
pub struct GeoPosCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}
impl Command for GeoPosCmd {
    fn execute(&self) -> String {
        db::geo_pos(self)
    }
}

#[derive(Debug)]
pub struct GeoRadiusByMemberCmd {
    pub arg_key: String,
    pub member: String,
    pub arg_radius: f64,
    pub arg_unit: Units,
    pub arg_order: ArgOrder,
}
impl Command for GeoRadiusByMemberCmd {
    fn execute(&self) -> String {
        db::geo_radius_by_member(self)
    }
}



#[derive(Debug)]
pub struct GeoDistCmd {
    pub arg_key: String,
    pub arg_mem_1: String,
    pub arg_mem_2: String,
    pub arg_unit: Units,
}

impl Command for GeoDistCmd {
    fn execute(&self) -> String {
        db::geo_dist(self)
    }
}

#[derive(Debug)]
pub struct GeoDelCmd {
    pub arg_key: String,
}

impl Command for GeoDelCmd {
    fn execute(&self) -> String {
        db::geo_del(self)
    }
}


#[derive(Debug)]
pub struct GeoRemoveCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}
impl Command for GeoRemoveCmd {
    fn execute(&self) -> String {
        db::geo_remove(self)
    }
}


#[derive(Debug)]
pub struct GeoJsonCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}
impl Command for GeoJsonCmd {
    fn execute(&self) -> String {
        db::geo_json(self)
    }
}

// server commands
#[derive(Debug)]
pub struct PingCmd;
impl Command for PingCmd {
    fn execute(&self) -> String {
        printer::print_pong()
    }
}
#[derive(Debug)]
pub struct LastSaveCmd;
impl Command for LastSaveCmd {
    fn execute(&self) -> String {
        db::last_save(self)
    }
}

#[derive(Debug)]
pub struct InfoCmd;
impl Command for InfoCmd {
    fn execute(&self) -> String {
        db::info(self)
    }
}

// json commands
#[derive(Debug)]
pub struct JSetRawCmd {
    pub arg_key: String,
    pub arg_value: String
}
impl Command for JSetRawCmd {
    fn execute(&self) -> String {
        db::jset_raw(self)
    }
}

pub type JSetArgItem = (String, Value);
#[derive(Debug)]
pub struct JSetCmd {
    pub arg_key: String,
    pub arg_set_items : Vec<JSetArgItem>
}
impl Command for JSetCmd {
    fn execute(&self) -> String {
        db::jset(self)
    }
}

#[derive(Debug)]
pub struct JMergeCmd {
    pub arg_key: String,
    pub arg_value: String
}
impl Command for JMergeCmd {
    fn execute(&self) -> String {
        db::jmerge(self)
    }
}

#[derive(Debug)]
pub struct JGetCmd {
    pub arg_key: String,
    pub arg_dot_path : Option<String>
}
impl Command for JGetCmd {
    fn execute(&self) -> String {
        db::jget(self)
    }
}

#[derive(Debug)]
pub struct JPathCmd {
    pub arg_key: String,
    pub arg_selector: String
}
impl Command for JPathCmd {
    fn execute(&self) -> String {
        db::jpath(self)
    }
}

#[derive(Debug)]
pub struct JDelCmd {
    pub arg_key: String
}
impl Command for JDelCmd {
    fn execute(&self) -> String {
        db::jdel(self)
    }
}

#[derive(Debug)]
pub struct JIncrByCmd {
    pub arg_key: String,
    pub arg_path : String,
    pub arg_increment_value : i64
}
impl Command for JIncrByCmd {
    fn execute(&self) -> String {
        db::jincrby(self)
    }
}


#[derive(Debug)]
pub struct JIncrByFloatCmd {
    pub arg_key: String,
    pub arg_path : String,
    pub arg_increment_value : f64
}
impl Command for JIncrByFloatCmd {
    fn execute(&self) -> String {
        db::jincrbyfloat(self)
    }
}
























