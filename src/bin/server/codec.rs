use std::io;
use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};
use resp::{Value, Decoder as RespDecoder};
use std::io::BufReader;
use std::str;
use redis_protocol::prelude::*;

pub struct RespCodec;

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
            buf.split_to(consumed);
            Ok(Some(frame))
        } else {
            Ok(None)
        };
    }
}
impl Encoder<Frame> for RespCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> io::Result<()> {
        encode_bytes( dst, &item);
        Ok(())
    }
}