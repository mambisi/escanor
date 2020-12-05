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
use async_raft::raft::{VoteRequest, InstallSnapshotRequest, AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotResponse, VoteResponse, ClientWriteRequest, ClientWriteResponse, ConflictOpt};

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

use serde::{Serialize, Deserialize};

use crate::command::Command;
use tracing::{debug, error, info, span, warn, Level};


#[derive(Clone, Debug, Serialize, Deserialize)]
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

use crate::storage;
use proto::rpc_service_client::RpcServiceClient;
use proto::{AppendEntriesReq, InstallSnapshotReq, VoteReq};

#[async_trait]
impl RaftNetwork<ClientRequest> for Network {
    async fn append_entries(&self, target: u64, rpc: AppendEntriesRequest<ClientRequest>) -> Result<AppendEntriesResponse> {
        let node_id = storage::get_node_addrs(target)?;
        let mut client = RpcServiceClient::connect(node_id).await?;
        let entries = bincode::serialize(&rpc.entries)?;
        let req = AppendEntriesReq {
            term: rpc.term,
            leader_id: rpc.leader_id,
            prev_log_index: rpc.prev_log_index,
            prev_log_term: rpc.prev_log_term,
            entries,
            leader_commit: rpc.leader_commit,
        };
        let resp = client.append_entries(req).await?;

        let r = resp.get_ref();
        let conflict_opt = match &r.conflict_opt {
            None => {
                None
            }
            Some(co) => {
                Some(ConflictOpt {
                    term: co.term,
                    index: co.index,
                })
            }
        };
        Ok(AppendEntriesResponse {
            term: r.term,
            success: r.success,
            conflict_opt,
        })
    }

    async fn install_snapshot(&self, target: u64, rpc: InstallSnapshotRequest) -> Result<InstallSnapshotResponse> {
        let node_id = storage::get_node_addrs(target)?;
        let mut client = RpcServiceClient::connect(node_id).await?;

        let req = InstallSnapshotReq {
            term: rpc.term,
            leader_id: rpc.leader_id,
            last_included_index: rpc.last_included_index,
            last_included_term: rpc.last_included_term,
            offset: rpc.offset,
            data: rpc.data,
            done: rpc.done,
        };

        let resp = client.install_snapshot(req).await?;
        let r = resp.get_ref();

        Ok(InstallSnapshotResponse {
            term: r.term
        })
    }

    async fn vote(&self, target: u64, rpc: VoteRequest) -> Result<VoteResponse> {
        let node_id = storage::get_node_addrs(target)?;
        let mut client = RpcServiceClient::connect(node_id).await?;

        let req = VoteReq {
            term: rpc.term,
            candidate_id: rpc.candidate_id,
            last_log_index: rpc.last_log_index,
            last_log_term: rpc.last_log_term,
        };

        let resp = client.vote(req).await?;
        let r = resp.get_ref();

        Ok(VoteResponse {
            term: r.term,
            vote_granted: r.vote_granted,
        })
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
                        context: context.clone(),
                        frame,
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