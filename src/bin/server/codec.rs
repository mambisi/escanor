use std::io;
use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};
use serde::{Serialize, Deserialize, Serializer, Deserializer};


use redis_protocol::prelude::*;
use async_raft::{AppData, AppDataResponse};
use crate::network::Context;
use std::sync::{Arc, RwLock};

pub struct RespCodec;

#[derive( Debug, Clone,Serialize, Deserialize)]
pub struct ClientRequest {
    #[serde(skip_serializing)]
    pub context : Arc<RwLock<Context>>,
    pub frame : Frame
}

#[derive( Debug, Clone,Serialize, Deserialize)]
pub struct ServerResponse {
    pub frame : Frame
}

impl AppData for ClientRequest {}
impl AppDataResponse for ServerResponse {}



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
            Err(_e) => return Ok(None)
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



