use console::style;

extern crate tokio;
extern crate tokio_util;

use tokio::net::{TcpListener, TcpStream};
//use tokio::prelude::*;
use crate::command;




use crate::printer;
use crate::printer::{print_from_error};

use futures::SinkExt;
use tokio::stream::StreamExt;
use tokio_util::codec::{BytesCodec, Decoder, LinesCodec, Framed};






use crate::codec::RespCodec;


use bytes::{BytesMut};
use nom::AsBytes;
use redis_protocol::prelude::*;
use redis_protocol::types::Frame;

use crate::command::Command;

struct Context{

}


fn process_socket(socket: TcpStream){
    // do work with socket here
    tokio::spawn(async move {
        let mut lines = RespCodec.framed(socket);
        while let Some(message) = lines.next().await {
            match message {
                Ok(frame) => {
                   let response_message = match command::compile_frame(frame) {
                        Ok(cmd) => {
                            let res = cmd.execute().to_owned();
                            res
                        },
                        Err(err) => {
                            print_from_error(&err)
                        },
                    };
                    let buf: BytesMut = BytesMut::from(response_message.as_bytes());
                    let (f,_) = decode_bytes(&buf).unwrap();
                    lines.send(f.unwrap_or(Frame::Error("Internal Error".to_owned()))).await;
                    //lines.send(Frame::SimpleString("Ok".to_string())).await;
                }
                Err(err) => { println!("Socket closed with error: {:?}", err); }
            };
        };
    });
}

pub async fn start_up(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind(addr).await?;

    printer::print_app_info();

    info!("{}", style("Server initialized").green());
    info!("Ready to accept connections");

    loop {
        match listener.accept().await {
            Ok((socket, _addr)) => {
                process_socket(socket);
            }
            Err(e) => error!("couldn't get client: {:?}", e),
        };
    }
}