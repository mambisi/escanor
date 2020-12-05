use console::style;

extern crate tokio;
extern crate tokio_util;

use tokio::net::{TcpListener, TcpStream};
//use tokio::prelude::*;
use crate::{command, EscanorRaft};
use crate::printer;
use crate::printer::{print_from_error};

use futures::SinkExt;
use tokio::stream::StreamExt;
use tokio_util::codec::{BytesCodec, Decoder, LinesCodec, Framed};

use crate::codec::{RespCodec, ClientRequest, ServerResponse};
use std::net::{SocketAddr, Shutdown};
use serde_yaml::Value;
use crate::config;
use async_raft::raft::{VoteRequest, InstallSnapshotRequest, AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotResponse, VoteResponse, ClientWriteRequest, ClientWriteResponse};

use bytes::{BytesMut};
use nom::AsBytes;
use redis_protocol::prelude::*;
use redis_protocol::types::Frame;

use async_raft::{RaftNetwork, Raft, ClientWriteError};
use anyhow::Result;
use anyhow::Error;

use async_trait::async_trait;
use std::sync::{Arc, RwLock};
use crate::storage::Storage;

use serde::{Serialize,Deserialize};

use crate::command::Command;
use tracing::{debug, error, info, span, warn, Level};


#[derive(Clone,Debug,Serialize, Deserialize)]
pub struct Context {
    pub client_addr: String,
    pub auth_is_required: bool,
    pub auth_key: Option<String>,
    pub client_authenticated: bool,
    pub client_auth_key: Option<String>,
}


#[derive(Clone, Debug)]
pub struct Network;

impl Network {
    pub fn new() -> Self {
        return Network {};
    }
}
use crate::RAFT;



#[async_trait]
impl RaftNetwork<ClientRequest> for Network {
    async fn append_entries(&self, target: u64, rpc: AppendEntriesRequest<ClientRequest>) -> Result<AppendEntriesResponse> {
        unimplemented!()
    }

    async fn install_snapshot(&self, target: u64, rpc: InstallSnapshotRequest) -> Result<InstallSnapshotResponse> {
        unimplemented!()
    }

    async fn vote(&self, target: u64, rpc: VoteRequest) -> Result<VoteResponse> {
        unimplemented!()
    }
}



fn process_socket(socket: TcpStream) {
    // do work with socket here
    tokio::spawn(async move {
        let addrs: SocketAddr = socket.peer_addr().unwrap();

        let conf_file = config::conf();

        let auth_key = match &conf_file.server {
            None => {
                String::new()
            }
            Some(server) => {
                match &server.require_auth {
                    None => {
                        String::new()
                    }
                    Some(auth) => {
                        auth.to_owned()
                    }
                }
            }
        };

        let context = Arc::new(RwLock::new(
            Context {
                client_addr: (addrs.ip().to_string()),
                auth_is_required: !auth_key.is_empty(),
                auth_key: if auth_key.is_empty() { None } else { Some(auth_key) },
                client_authenticated: false,
                client_auth_key: None,
            }
        ));

        let mut lines = RespCodec.framed(socket);
        while let Some(message) = lines.next().await {
            match message {
                Ok(frame) => {
                    let r = RAFT.client_write(ClientWriteRequest::new(ClientRequest {
                        context : context.clone(),
                        frame
                    })).await;

                    match r {
                        Ok(response) => {
                            let f = response.data.frame;
                            /*
                            let buf: BytesMut = BytesMut::from(response_message.as_bytes());
                            let (f, _) = decode_bytes(&buf).unwrap();
                             */
                            lines.send(f).await;
                        }
                        Err(e) => {
                            debug!("Write Error: {:?}", e);
                            lines.send(Frame::Error("SERVER ERROR".to_owned())).await;
                        }
                    };
                }
                Err(err) => {
                    debug!("Disconnected Context");
                    println!("Socket closed with error: {:?}", err);
                }
            };
        };
    });
}


pub async fn start_up(addr: &str) -> Result<()> {


    let mut listener = TcpListener::bind(addr).await?;
    printer::print_app_info();


    info!("{}", style("Server initialized").green());
    loop {
        match listener.accept().await {
            Ok((socket, _addr)) => {
                process_socket(socket);
            }
            Err(e) => error!("couldn't get client: {:?}", e),
        };
    }
}