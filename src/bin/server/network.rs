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

#[derive(Clone,Debug)]
pub struct Context{
    pub db : Option<String>,
    pub client_addr : SocketAddr,
    pub auth_is_required : bool,
    pub auth_key : Option<String>,
    pub client_authenticated : bool,
    pub client_auth_key : Option<String>
}

use std::net::{SocketAddr,Shutdown};
use futures::io::Error;
use serde_yaml::Value;
use crate::config;

fn process_socket(socket: TcpStream){
    // do work with socket here
    tokio::spawn(async move {

        let addrs : SocketAddr = socket.peer_addr().unwrap();

        let conf_file = config::conf();

        let auth_key = match &conf_file.server{
            None => {
                String::new()
            },
            Some(server) => {
                match &server.require_auth {
                    None => {
                        String::new()
                    },
                    Some(auth) => {
                        auth.to_owned()
                    },
                }
            },
        };

        let mut context = Context {
            db : None,
            client_addr: addrs,
            auth_is_required : !auth_key.is_empty(),
            auth_key: if auth_key.is_empty() {None}else { Some(auth_key) },
            client_authenticated : false,
            client_auth_key : None
        };

        let mut lines = RespCodec.framed(socket);
        while let Some(message) = lines.next().await {
            match message {
                Ok(frame) => {
                   let response_message = match command::compile_frame(frame) {
                        Ok(cmd) => {
                            let res = cmd.execute(&mut context).to_owned();
                            res
                        },
                        Err(err) => {
                            print_from_error(&err)
                        },
                    };
                    let buf: BytesMut = BytesMut::from(response_message.as_bytes());
                    let (f,_) = decode_bytes(&buf).unwrap();
                    lines.send(f.unwrap_or(Frame::Error("Internal Error".to_owned()))).await;
                }
                Err(err) => {
                    debug!("Disconnected Context: {:?}", context);
                    println!("Socket closed with error: {:?}", err); }
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