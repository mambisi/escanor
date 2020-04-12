
extern crate redis_protocol;
extern crate bytes;
extern crate escanor;

use bytes::BytesMut;
use redis_protocol::prelude::*;


use escanor::common::parser;
pub fn generate_tokens_from_resp(buf: &[u8]) -> Vec<String> {
    let mut tokens: Vec<String> = vec![];

    let buf: BytesMut = BytesMut::from(buf);

    let (frame, consumed) = match decode_bytes(&buf) {
        Ok((f, c)) => (f, c),
        Err(e) => {
            return tokens;
        }
    };

    let frame = frame.unwrap();

    let req = match frame {
        Frame::Array(a) => {
            a
        }
        _ => {
            return tokens;
        }
    };

    for f in req {
        match f {
            Frame::SimpleString(s) => {
                tokens.push(s)
            }
            Frame::Integer(i) => {
                tokens.push(i.to_string())
            }
            Frame::BulkString(s) => {
                let st = String::from_utf8(s).unwrap_or("".to_owned());
                tokens.push(st)
            }
            _ => {}
        }
    }

    return tokens;
}


pub fn generate_tokens(cmd: &[u8]) -> Vec<String> {
    parser::parse_raw_cmd(cmd)
}