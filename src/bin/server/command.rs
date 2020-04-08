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


pub fn parse(cmd: &String) -> Result<Box<dyn Command>, error::SyntaxError> {
    //todo add redis command parser
    let tokens: Vec<String> = tokenizer::generate_tokens(cmd);
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
pub struct KeysCmd;


#[derive(Debug)]
pub struct PingCmd;

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
        db::list_keys(self)
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
