extern crate regex;

use crate::{db, printer};
use crate::error;

use crate::error::SyntaxError;
use crate::tokenizer;
use crate::syntax_analyzer;


use crate::unit_conv::Units;

use redis_protocol::types::Frame;
use serde_json::Value;
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
use crate::db::Data;

pub trait Command {
    //fn execute(&self, db: &db::DB);
    fn execute(&self, context: &mut Context) -> String;
}

pub fn auth_context<T>(context: &mut Context, fn_args: T, f: fn(context : &Context,T) -> String) -> String {
    if !context.auth_is_required {
        return f(context,fn_args);
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
        f(context, fn_args)
    } else {
        print_err("ERR auth failed")
    };
}

/// Creates an implementation for Command for a type with in a auth context
macro_rules! cmd_with_context_impl {
    ($type : ty => $func : path) => {
        impl Command for $type {
            fn execute(&self, context: &mut Context) -> String {
                auth_context(context,self,$func)
            }
        }
    };
}
/// Creates a command struct with a context implementation
macro_rules! make_command {
    ($name : ident {$($arg : ident : $arg_type : ty ),+} -> $func : path) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $arg : $arg_type),+
        }
        cmd_with_context_impl!{$name => $func}
    };
    ($name : ident; -> $func : path) => {
        #[derive(Debug)]
        pub struct $name;
        cmd_with_context_impl!{$name => $func}
    };
    ($name : ident {$($arg : ident : $arg_type : ty ),+}) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $arg : $arg_type),+
        }
    };
    ($name : ident;) => {
        #[derive(Debug)]
        pub struct $name;
    };
}

#[derive(Debug, Clone, Copy)]
pub enum ArgOrder {
    ASC,
    DESC,
    UNSPECIFIED,
}
pub type CmdGeoItem = (f64, f64, String);

pub type JSetArgItem = (String, Value);

make_command!(PingCmd;);
make_command!(AuthCmd {arg_password : String});
/*
make_command!(LastSaveCmd; -> db::last_save);
make_command!(BGSaveCmd; -> db::bg_save );
make_command!(FlushDBCmd; -> db::flush_db);
 */
make_command!(RandomKeyCmd; -> db::random_key);
make_command!(InfoCmd; -> db::info);
make_command!(DBSizeCmd; -> db::db_size);

impl Command for PingCmd {
    fn execute(&self, _: &mut Context) -> String {
        printer::print_pong()
    }
}
impl Command for AuthCmd {
    fn execute(&self, context: &mut Context) -> String {
        db::auth(context, self)
    }
}

//Key Value Commands
make_command!(SetCmd{arg_key : String,arg_value : Data, arg_exp : u32} -> db::set);
make_command!(GetSetCmd{arg_key : String, arg_value : Data} -> db::get_set);
make_command!(GetCmd{arg_key : String} -> db::get);
make_command!(DelCmd{arg_key : String} -> db::del);
make_command!(PersistCmd{arg_key : String} -> db::persist);
make_command!(TTLCmd{arg_key : String} -> db::ttl);
make_command!(ExpireCmd{arg_key: String, arg_value : i64} -> db::expire);
make_command!(IncrByCmd{arg_key: String, arg_value : i64} -> db::incr_by);
make_command!(ExpireAtCmd{arg_key: String, arg_value : i64} -> db::expire_at);
make_command!(KeysCmd{pattern : String} -> db::keys);
make_command!(ExistsCmd{keys : Vec<String>} -> db::exists);
// Geo Spatial Commands
make_command!(GeoAddCmd{arg_key : String, items : Vec<CmdGeoItem>} -> db::geo_add);
make_command!(GeoRadiusCmd{arg_key: String, arg_lng: f64,arg_lat: f64,arg_radius: f64,arg_unit: Units,arg_order: ArgOrder} -> db::geo_radius);
make_command!(GeoHashCmd{arg_key : String, items : Vec<String>} -> db::geo_hash);
make_command!(GeoPosCmd{arg_key : String, items : Vec<String>} -> db::geo_pos);
make_command!(GeoRadiusByMemberCmd{arg_key: String,member: String,arg_radius: f64,arg_unit: Units,arg_order: ArgOrder} -> db::geo_radius_by_member);
make_command!(GeoDistCmd{arg_key: String,arg_mem_1: String,arg_mem_2: String,arg_unit: Units} -> db::geo_dist);
make_command!(GeoDelCmd{arg_key: String} -> db::geo_del);
make_command!(GeoRemoveCmd{arg_key : String, items : Vec<String>} -> db::geo_remove);
make_command!(GeoJsonCmd{arg_key : String,items : Vec<String>} -> db::geo_json);
// json commands
make_command!(JSetRawCmd{arg_key : String, arg_value: String} -> db::jset_raw);
make_command!(JSetCmd{arg_key : String, arg_set_items : Vec<JSetArgItem>} -> db::jset);
make_command!(JMergeCmd{arg_key : String,  arg_value : String} -> db::jmerge);
make_command!(JGetCmd{arg_key : String, arg_dot_path : Option<String>} -> db::jget);
make_command!(JPathCmd{arg_key : String, arg_selector : String} -> db::jpath);
make_command!(JDelCmd{arg_key :String} -> db::jdel);
make_command!(JRemCmd{arg_key : String, arg_paths : Vec<String>} -> db::jrem);
make_command!(JIncrByCmd{arg_key: String, arg_path: String,arg_increment_value: i64} -> db::jincr_by);
make_command!(JIncrByFloatCmd{arg_key: String,arg_path: String,arg_increment_value: f64} -> db::jincr_by_float);
