extern crate regex;

use crate::db;
use crate::db::{DataType, ESRecord};
use crate::error;
use std::rc::Rc;
use std::sync::Arc;
use crate::error::SyntaxError;
use std::borrow::Borrow;

use regex::Regex;
use std::collections::BTreeMap;

const DATA_TYPES: [&str; 4] = ["string", "int", "json", "point"];

pub fn parse(cmd: &String) -> Result<Box<dyn Command>, error::SyntaxError> {
    let tokens: Vec<String> = tokenize(cmd.as_str());
    match analyse_syntax(tokens) {
        Ok(t) => Ok(t),
        Err(e) => Err(SyntaxError)
    }
}

fn analyse_syntax(tokens: Vec<String>) -> Result<Box<dyn Command>, error::SyntaxError> {
    let default_value: String = String::from("");
    let default_type: String = String::from("string");
    let default_exp_time_str: String = String::from("0");

    let mut itr = tokens.iter();
    let cmd = itr.next().unwrap_or(&default_value).to_lowercase();
    if cmd.eq("") {
        return Err(error::SyntaxError);
    }

    if cmd == "set" {
        let arg_key = itr.next().unwrap_or(&default_value);
        if arg_key.eq("") { return Err(error::SyntaxError); }

        let mut arg_value: String = String::from("");

        let mut arg_type = itr.next().unwrap_or(&default_value);
        if arg_type == "" { return Err(error::SyntaxError); } else if is_type_valid(arg_type) {} else {
            arg_value = arg_type.to_owned();
            arg_type = &default_type
        }

        if arg_value.is_empty() {
            arg_value = itr.next().unwrap_or(&default_value).to_owned();
        }

        let arg_ex_cmd = itr.next().unwrap_or(&default_value).as_str();

        if arg_ex_cmd == "" {

            return Ok(Box::new(SetCmd {
                arg_key: arg_key.to_owned(),
                arg_type: get_type(arg_type),
                arg_value,
                arg_exp: 0,
            }));
        } else if arg_ex_cmd == "ex" {
            let arg_next = itr.next().unwrap_or(&default_exp_time_str);
            let arg_exp = arg_next.parse::<u32>().unwrap_or(0);
            return Ok(Box::new(SetCmd {
                arg_key: arg_key.to_owned(),
                arg_type: get_type(arg_type),
                arg_value,
                arg_exp,
            }));
        }
    }
    else if cmd == "get" {
        let arg_key = itr.next().unwrap_or(&default_value);
        if arg_key.eq("") { return Err(error::SyntaxError); }
        return Ok(Box::new(GetCmd {
            arg_key : arg_key.to_owned()
        }));
    }
    else if cmd == "del" {
        let arg_key = itr.next().unwrap_or(&default_value);
        if arg_key.eq("") { return Err(error::SyntaxError); }
        return Ok(Box::new(DelCmd {
            arg_key : arg_key.to_owned()
        }));
    }
    else if cmd == "keys" {
        return Ok(Box::new(KeysCmd));
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

// Grammar > set [key] string|number|json [value] ex [exp]
#[derive(Debug)]
pub struct SetCmd {
    pub arg_key: String,
    pub arg_type: db::DataType,
    pub arg_value: String,
    pub arg_exp: u32,
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


fn is_type_valid(t: &String) -> bool {
    for i in DATA_TYPES.iter() {
        if t.eq(i) {
            return true;
        }
    }
    false
}

fn get_type(t: &String) -> db::DataType {
    if t.eq("string") {
        return db::DataType::String;
    } else if t.eq("int") {
        return db::DataType::Integer;
    } else if t.eq("json") {
        return db::DataType::Json;
    } else if t.eq("point") {
        return db::DataType::Point;
    }
    return db::DataType::String;
}


impl Command for SetCmd {
    fn execute(&self) -> String{
        db::set(self)
    }
}

impl Command for GetCmd {
    fn execute(&self) -> String{
        db::get(self)
    }
}

impl Command for DelCmd {
    fn execute(&self) -> String{
        db::del(self)
    }
}

impl Command for KeysCmd {
    fn execute(&self) -> String{
        db::list_keys(self)
    }
}

#[test]
fn set_command_test_valid_with_expiration(){
    let ucmd = String::from(r##"`set` `name` `{"name" : "json"}`"##);
    match parse(&ucmd){
        Ok(c) => {
            c.execute();
            assert!(true)
        },
        Err(e) => assert!(false,e.to_string())
    };
}