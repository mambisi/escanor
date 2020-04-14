use std::io;
use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};
use resp::{Value, Decoder as RespDecoder};
use std::io::BufReader;
use std::str;

pub struct RespCodec;

/*
impl Decoder for RespCodec {
    type Item = Value;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<String>> {
        let s = if let Some(n) = buf.iter().rposition(|b| *b == b'\n') {
            let client_query = buf.split_to(n + 1);

            match str::from_utf8(&client_query.as_ref()) {
                Ok(s) => s.to_string(),
                Err(_) => return Err(io::Error::new(io::ErrorKind::Other, "invalid string")),
            }
        } else {
            return Ok(None);
        };
        println!("{:?}", s);
        Ok(Some(s))
    }
}*/

use redis_protocol::prelude::*;

impl Decoder for RespCodec {
    // ...
    type Item = Frame;
    type Error = io::Error;


    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Frame>> {
        if buf.is_empty() {
            return Ok(None);
        }
        let (frame, consumed) = match decode_bytes(&buf) {
            Ok((f, c)) => (f, c),
            Err(e) => return Ok(None)
        };

        return if let Some(frame) = frame {
            //println!("Parsed frame {:?} and consumed {} bytes", frame, consumed);
            //buf.split_to();
            buf.split_to(consumed);
            Ok(Some(frame))
        } else {
            //println!("Incomplete frame, parsed {} bytes", consumed);
            Ok(None)
        };
        //Ok(Some(Frame::SimpleString("Ok".to_owned())))
    }
}
impl Encoder<Frame> for RespCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> io::Result<()> {
        encode_bytes( dst, &item);
        Ok(())
    }
}