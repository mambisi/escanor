extern crate regex;

use crate::{db, printer};
use crate::error;

use crate::error::SyntaxError;
use crate::tokenizer;
use crate::syntax_analyzer;


use crate::unit_conv::Units;

use redis_protocol::types::Frame;
use serde_json::Value;
use crate::db::{ESValue, bg_save};
use crate::printer::*;

pub fn compile_frame(frame: Frame) -> Result<Box<dyn Command>, error::SyntaxError> {
    let tokens: Vec<String> = tokenizer::generate_token_from_frame(frame);
    match syntax_analyzer::analyse_token_stream(tokens) {
        Ok(t) => Ok(t),
        Err(_e) => Err(SyntaxError)
    }
}

pub fn compile(buf: &[u8]) -> Result<Box<dyn Command>, error::SyntaxError> {
    let _empty_string = String::new();
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
        Err(_e) => Err(SyntaxError)
    }
}

pub fn compile_resp(buf: &[u8]) -> Result<Box<dyn Command>, error::SyntaxError> {
    let tokens: Vec<String> = tokenizer::generate_tokens_from_resp(buf);
    match syntax_analyzer::analyse_token_stream(tokens) {
        Ok(t) => Ok(t),
        Err(_e) => Err(SyntaxError)
    }
}


use crate::network::Context;

pub trait Command {
    //fn execute(&self, db: &db::DB);
    fn execute(&self, context: &mut Context) -> String;
}

pub fn auth_context<T>(context: &mut Context,fn_args : T, f : fn( T ) -> String ) -> String {
    if !context.auth_is_required {
        return f(fn_args)
    }

    let auth_key = match &context.auth_key {
        Some(k) => k.to_owned(),
        None => {
            return print_err("ERR auth");
        }
    };

    let client_auth_key = match &context.client_auth_key {
        Some(k) => k.to_owned(),
        None => {
            return print_err("ERR auth");
        }
    };

    if auth_key == client_auth_key {
        context.client_authenticated = true
    } else {
        context.client_authenticated = false
    }
    return if context.client_authenticated {
        f(fn_args)
    } else {
        print_err("ERR auth failed")
    }
}
// server commands
#[derive(Debug)]
pub struct PingCmd;

impl Command for PingCmd {
    fn execute(&self, context: &mut Context) -> String {
        printer::print_pong()
    }
}

#[derive(Debug)]
pub struct LastSaveCmd;

impl Command for LastSaveCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::last_save)
    }
}

#[derive(Debug)]
pub struct AuthCmd {
    pub arg_password : String
}

impl Command for AuthCmd {
    fn execute(&self, context: &mut Context) -> String {
        db::auth(context,self)
    }
}

#[derive(Debug)]
pub struct BGSaveCmd;

impl Command for BGSaveCmd {
    fn execute(&self, context: &mut Context) -> String {
        //db::bg_save(self)
        auth_context(context,self,bg_save)
    }
}

#[derive(Debug)]
pub struct FlushDBCmd;

impl Command for FlushDBCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::flush_db)
    }
}

#[derive(Debug)]
pub struct RandomKeyCmd;

impl Command for RandomKeyCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::random_key)
    }
}

#[derive(Debug)]
pub struct InfoCmd;

impl Command for InfoCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::info)
    }
}

#[derive(Debug)]
pub struct DBSizeCmd;

impl Command for DBSizeCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::db_size)
    }
}


// Key value Commands
#[derive(Debug)]
pub struct SetCmd {
    pub arg_key: String,
    pub arg_value: ESValue,
    pub arg_exp: u32,
}

impl Command for SetCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::set)
    }
}

#[derive(Debug)]
pub struct GetSetCmd {
    pub arg_key: String,
    pub arg_value: ESValue,
}

impl Command for GetSetCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::get_set)
    }
}

#[derive(Debug)]
pub struct GetCmd {
    pub arg_key: String
}

impl Command for GetCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::get)
    }
}

#[derive(Debug)]
pub struct DelCmd {
    pub arg_key: String
}

impl Command for DelCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::del)
    }
}

#[derive(Debug)]
pub struct PersistCmd {
    pub arg_key: String
}

impl Command for PersistCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::persist)
    }
}

#[derive(Debug)]
pub struct TTLCmd {
    pub arg_key: String
}

impl Command for TTLCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::ttl)
    }
}

#[derive(Debug)]
pub struct ExpireCmd {
    pub arg_key: String,
    pub arg_value: i64,
}

impl Command for ExpireCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::expire)
    }
}

#[derive(Debug)]
pub struct ExpireAtCmd {
    pub arg_key: String,
    pub arg_value: i64,
}

impl Command for ExpireAtCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::expire_at)
    }
}

#[derive(Debug)]
pub struct KeysCmd {
    pub pattern: String
}

impl Command for KeysCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::keys)
    }
}

#[derive(Debug)]
pub struct ExistsCmd {
    pub keys: Vec<String>,
}

impl Command for ExistsCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::exists)
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
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::geo_add)
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
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::geo_radius)
    }
}

#[derive(Debug)]
pub struct GeoHashCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}

impl Command for GeoHashCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::geo_hash)
    }
}


#[derive(Debug)]
pub struct GeoPosCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}

impl Command for GeoPosCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::geo_pos)
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
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::geo_radius_by_member)
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
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::geo_dist)
    }
}

#[derive(Debug)]
pub struct GeoDelCmd {
    pub arg_key: String,
}

impl Command for GeoDelCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::geo_del)
    }
}


#[derive(Debug)]
pub struct GeoRemoveCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}

impl Command for GeoRemoveCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::geo_remove)
    }
}


#[derive(Debug)]
pub struct GeoJsonCmd {
    pub arg_key: String,
    pub items: Vec<String>,
}

impl Command for GeoJsonCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::geo_json)
    }
}


// json commands
#[derive(Debug)]
pub struct JSetRawCmd {
    pub arg_key: String,
    pub arg_value: String,
}

impl Command for JSetRawCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::jset_raw)
    }
}

pub type JSetArgItem = (String, Value);

#[derive(Debug)]
pub struct JSetCmd {
    pub arg_key: String,
    pub arg_set_items: Vec<JSetArgItem>,
}

impl Command for JSetCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::jset)
    }
}

#[derive(Debug)]
pub struct JMergeCmd {
    pub arg_key: String,
    pub arg_value: String,
}

impl Command for JMergeCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::jmerge)
    }
}

#[derive(Debug)]
pub struct JGetCmd {
    pub arg_key: String,
    pub arg_dot_path: Option<String>,
}

impl Command for JGetCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::jget)
    }
}

#[derive(Debug)]
pub struct JPathCmd {
    pub arg_key: String,
    pub arg_selector: String,
}

impl Command for JPathCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::jpath)
    }
}

#[derive(Debug)]
pub struct JDelCmd {
    pub arg_key: String
}

impl Command for JDelCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::jdel)
    }
}

#[derive(Debug)]
pub struct JIncrByCmd {
    pub arg_key: String,
    pub arg_path: String,
    pub arg_increment_value: i64,
}

impl Command for JIncrByCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::jincr_by)
    }
}


#[derive(Debug)]
pub struct JIncrByFloatCmd {
    pub arg_key: String,
    pub arg_path: String,
    pub arg_increment_value: f64,
}

impl Command for JIncrByFloatCmd {
    fn execute(&self, context: &mut Context) -> String {
        auth_context(context,self,db::jincr_by_float)
    }
}
























