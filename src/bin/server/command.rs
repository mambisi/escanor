extern crate regex;

use crate::{db, printer, unit_conv};
use crate::error;
use crate::util;
use crate::error::SyntaxError;

use regex::Regex;
use serde::export::Option::Some;
use crate::unit_conv::Units;


pub fn parse(cmd: &String) -> Result<Box<dyn Command>, error::SyntaxError> {
    let tokens: Vec<String> = tokenize(cmd.as_str());
    match analyse_syntax(tokens) {
        Ok(t) => Ok(t),
        Err(e) => Err(SyntaxError)
    }
}

fn analyse_syntax(tokens: Vec<String>) -> Result<Box<dyn Command>, error::SyntaxError> {
    let empty_string: String = String::from("");
    let default_type: String = String::from("string");
    let default_exp_time_str: String = String::from("0");

    let mut itr = tokens.iter();
    let cmd = itr.next().unwrap_or(&empty_string).to_lowercase();
    if cmd.eq("") {
        return Err(error::SyntaxError);
    }
    if cmd == "ping" {
        return Ok(Box::new(PingCmd));
    }

    if cmd == "set" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_value = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_ex_cmd = &itr.next().unwrap_or(&empty_string).to_lowercase();

        if arg_ex_cmd.is_empty() {
            return Ok(Box::new(SetCmd {
                arg_key: arg_key.to_owned(),
                arg_type: get_type(arg_value),
                arg_value: arg_value.to_owned(),
                arg_exp: 0,
            }));
        } else if arg_ex_cmd == "ex" {
            let arg_next = itr.next().unwrap_or(&default_exp_time_str);
            let arg_exp = arg_next.parse::<u32>().unwrap_or(0);
            return Ok(Box::new(SetCmd {
                arg_key: arg_key.to_owned(),
                arg_type: get_type(arg_value),
                arg_value: arg_value.to_owned(),
                arg_exp,
            }));
        }
    } else if cmd == "get" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(GetCmd {
            arg_key: arg_key.to_owned()
        }));
    } else if cmd == "del" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        return Ok(Box::new(DelCmd {
            arg_key: arg_key.to_owned()
        }));
    } else if cmd == "keys" {
        return Ok(Box::new(KeysCmd));
    }
    // GEOADD [key] long lat tag [long lat tag...]
    else if cmd == "geoadd" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        let mut items_after_key: Vec<&String> = vec![];

        while let Some(i) = itr.next() {
            items_after_key.push(i);
        }
        if items_after_key.is_empty() {
            return Err(error::SyntaxError);
        }
        if items_after_key.len() % 3 != 0 {
            return Err(error::SyntaxError);
        }
        //split items_after_key in arrays of [3, &String]
        let mut geo_point_chunks = items_after_key.chunks_exact(3);

        let mut items: Vec<CmdGeoItem> = vec![];

        while let Some(c) = geo_point_chunks.next() {
            let lng = c[0];
            let lat = c[1];
            let tag = c[2];

            if !(util::is_numeric(lat) && util::is_numeric(lng)) {
                return Err(error::SyntaxError);
            }

            let lat = lat.parse::<f64>().unwrap();
            let lng = lng.parse::<f64>().unwrap();
            let tag = tag.to_owned();

            items.push((lat, lng, tag))
        }

        return Ok(Box::new(GeoAddCmd {
            arg_key: arg_key.to_owned(),
            items,
        }));
    } else if cmd == "geohash" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }
        let mut items_after_key: Vec<String> = vec![];

        while let Some(i) = itr.next() {
            items_after_key.push(i.to_owned());
        }

        if items_after_key.is_empty() {
            return Err(error::SyntaxError);
        }

        return Ok(Box::new(GeoHashCmd {
            arg_key: arg_key.to_owned(),
            items: items_after_key,
        }));
    } else if cmd == "georadius" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_lng = itr.next().unwrap_or(&empty_string);
        if arg_lng.is_empty() { return Err(error::SyntaxError); }

        let arg_lat = itr.next().unwrap_or(&empty_string);
        if arg_lat.is_empty() { return Err(error::SyntaxError); }

        let arg_radius = itr.next().unwrap_or(&empty_string);
        if arg_radius.is_empty() { return Err(error::SyntaxError); }

        let arg_unit_string = &itr.next().unwrap_or(&empty_string).to_lowercase();
        if arg_unit_string.is_empty() { return Err(error::SyntaxError); }

        let arg_unit = match unit_conv::parse(arg_unit_string) {
            Ok(unit) => unit,
            Err(e) => {
                return Err(error::SyntaxError);
            }
        };

        let arg_order_string = itr.next().unwrap_or(&empty_string).to_lowercase();
        let mut arg_order = ArgOrder::UNSPECIFIED;

        match check_validate_arg_order(arg_order_string, &mut arg_order) {
            Ok(()) => (),
            Err(e) => {
                return Err(e);
            }
        };

        if !(util::is_numeric(arg_lng) && util::is_numeric(arg_lng) && util::is_numeric(arg_radius)) {
            return Err(error::SyntaxError);
        }

        let lat = arg_lat.parse::<f64>().unwrap();
        let lng = arg_lng.parse::<f64>().unwrap();
        let rads = arg_radius.parse::<f64>().unwrap();

        return Ok(Box::new(GeoRadiusCmd {
            arg_key: arg_key.to_owned(),
            arg_lng: lng,
            arg_lat: lat,
            arg_radius: rads,
            arg_unit,
            arg_order,
        }));
    } else if cmd == "geodist" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let member_1 = itr.next().unwrap_or(&empty_string);
        if member_1.is_empty() { return Err(error::SyntaxError); }

        let member_2 = itr.next().unwrap_or(&empty_string);
        if member_2.is_empty() { return Err(error::SyntaxError); }

        let arg_unit_string = &itr.next().unwrap_or(&empty_string).to_lowercase();
        if arg_unit_string.is_empty() { return Err(error::SyntaxError); }

        let arg_unit = match unit_conv::parse(arg_unit_string) {
            Ok(unit) => unit,
            Err(e) => {
                return Err(error::SyntaxError);
            }
        };

        return Ok(Box::new(GeoDistCmd {
            arg_key: arg_key.to_owned(),
            arg_mem_1: member_1.to_owned(),
            arg_mem_2: member_2.to_owned(),
            arg_unit,
        }));
    } else if cmd == "georadiusbymember" {
        let arg_key = itr.next().unwrap_or(&empty_string);
        if arg_key.is_empty() { return Err(error::SyntaxError); }

        let arg_member = itr.next().unwrap_or(&empty_string);
        if arg_member.is_empty() { return Err(error::SyntaxError); }

        let arg_radius = itr.next().unwrap_or(&empty_string);
        if arg_radius.is_empty() { return Err(error::SyntaxError); }

        let arg_unit_string = &itr.next().unwrap_or(&empty_string).to_lowercase();
        if arg_unit_string.is_empty() { return Err(error::SyntaxError); }

        let arg_unit = match unit_conv::parse(arg_unit_string) {
            Ok(unit) => unit,
            Err(e) => {
                return Err(error::SyntaxError);
            }
        };

        let arg_order_string = itr.next().unwrap_or(&empty_string).to_lowercase();
        let mut arg_order = ArgOrder::UNSPECIFIED;

        match check_validate_arg_order(arg_order_string, &mut arg_order) {
            Ok(()) => (),
            Err(e) => {
                return Err(e);
            }
        };

        if !(util::is_numeric(arg_radius)) {
            return Err(error::SyntaxError);
        }
        let rads = arg_radius.parse::<f64>().unwrap();

        return Ok(Box::new(
            GeoRadiusByMemberCmd{
                arg_key: arg_key.to_owned(),
                member: arg_member.to_string(),
                arg_radius: rads,
                arg_unit,
                arg_order
            }
        ))


    }

    Err(error::SyntaxError)
}

fn check_validate_arg_order(arg_order_string: String, arg_order: &mut ArgOrder) -> Result<(), error::SyntaxError> {
    if arg_order_string.is_empty() {
        return Ok(());
    } else if !arg_order_string.is_empty() && (arg_order_string == "asc" || arg_order_string == "desc") {
        *arg_order = match arg_order_string.as_str() {
            "asc" => ArgOrder::ASC,
            "desc" => ArgOrder::DESC,
            _ => {
                return Err(error::SyntaxError);
            }
        };
        return Ok(());
    }
    Err(error::SyntaxError)
}

fn tokenize(cmd: &str) -> Vec<String> {
    let mut tokens: Vec<String> = vec![];

    let cmd = cmd.trim();

    let mut block_seq = String::new();
    let mut in_string = false;
    let mut next_char = '\0';
    let mut prev_char = '\0';
    let text_qualifier = '`';
    let text_delimiter = ' ';

    for (i, current_char) in cmd.chars().enumerate() {
        let block = &mut block_seq;

        if i > 0 {
            prev_char = cmd.chars().nth(i - 1).unwrap();
        } else {
            prev_char = '\0';
        }

        if i + 1 > cmd.len() {
            next_char = cmd.chars().nth(i - 1).unwrap();
        } else {
            next_char = '\0';
        }

        if current_char == text_qualifier && (prev_char == '\0' || prev_char == text_delimiter) && !in_string {
            in_string = true;
            continue;
        }

        if current_char == text_qualifier && (next_char == '\0' || next_char == text_delimiter) && in_string {
            in_string = false;
            continue;
        }

        if current_char == text_delimiter && !in_string {
            let token = block.clone();
            tokens.push(token);
            block_seq.clear();
            continue;
        }

        block_seq.push(current_char);
    }
    tokens.push(block_seq);
    return tokens;
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
pub struct GeoDistCmd {
    pub arg_key: String,
    pub arg_mem_1: String,
    pub arg_mem_2: String,
    pub arg_unit: Units,
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


#[derive(Debug)]
pub struct GeoHashCmd {
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


fn get_type(t: &String) -> db::DataType {
    if util::is_integer(t) { db::DataType::Integer } else { db::DataType::String }
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
