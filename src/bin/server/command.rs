extern crate regex;

use crate::db;
use crate::db::{DataType, ESRecord};
use crate::error;
use crate::util;
use std::rc::Rc;
use std::sync::Arc;
use crate::error::SyntaxError;
use std::borrow::Borrow;

use regex::Regex;
use std::collections::BTreeMap;
use serde::export::Option::Some;


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
    // GEOADD [key] [number number tag] ... n
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
            let lat = c[0];
            let lng = c[1];
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


fn get_type(t: &String) -> db::DataType {
    if util::is_integer(t) { db::DataType::Integer } else { db::DataType::String }
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

#[test]
fn set_command_test_valid_with_expiration() {
    let ucmd = String::from(r##"`set` `name` `{"name" : "json"}`"##);
    match parse(&ucmd) {
        Ok(c) => {
            c.execute();
            assert!(true)
        }
        Err(e) => assert!(false, e.to_string())
    };
}

#[test]
fn set_command_geoadd() {
    let ucmd = String::from(r##"GEOADD stores 1 23.1 kumasi"##);
    match parse(&ucmd) {
        Ok(c) => {

            c.execute();
            assert!(true)
        }
        Err(e) => assert!(false, e.to_string())
    };
}

#[test]
fn set_command_geoadd_error() {
    let ucmd = String::from(r##"GEOADD stores k m m"##);
    match parse(&ucmd) {
        Ok(c) => {

            c.execute();
            assert!(false)
        }
        Err(e) => assert!(true, e.to_string())
    };
}