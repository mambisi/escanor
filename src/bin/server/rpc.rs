use proto::rpc_service_server::{RpcService, RpcServiceServer};
use tonic::{Status, Response, Request};
use proto::{InstallSnapshotResp, VoteReq, AppendEntriesResp, InstallSnapshotReq, AppendEntriesReq, VoteResp, ConflictOpt};
use anyhow::Result;
use crate::EscanorRaft;
use std::sync::Arc;
use tonic::transport::Server;
use async_raft::raft::{AppendEntriesRequest, InstallSnapshotRequest, VoteRequest};
use async_raft::raft::Entry;
use crate::codec::ClientRequest;
use crate::RAFT;


pub struct RPC;


#[tonic::async_trait]
impl RpcService for RPC {
    async fn append_entries(&self, request: Request<AppendEntriesReq>) -> Result<Response<AppendEntriesResp>, Status> {
        let r = request.get_ref();

        let entries: Vec<Entry<ClientRequest>> = bincode::deserialize(&r.entries)
            .or_else(|_| Err(Status::invalid_argument("Error parsing entries")))?;
        let rpc = AppendEntriesRequest {
            term: r.term,
            leader_id: r.leader_id,
            prev_log_index: r.prev_log_index,
            prev_log_term: r.prev_log_term,
            entries,
            leader_commit: r.leader_commit,
        };
        let resp = RAFT.append_entries(rpc).await.or_else(
            |err|  Err(Status::invalid_argument("Raft Error"))
        )?;
        let conflict_opt = match resp.conflict_opt {
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

        Ok(Response::new(
            AppendEntriesResp {
                term: resp.term,
                success: resp.success,
                conflict_opt,
            }
        ))
    }

    async fn install_snapshot(&self, request: Request<InstallSnapshotReq>) -> Result<Response<InstallSnapshotResp>, Status> {
        let r = request.get_ref();

        let rpc = InstallSnapshotRequest {
            term: r.term,
            leader_id: r.leader_id,
            last_included_index: r.last_included_index,
            last_included_term: r.last_included_term,
            offset: r.offset,
            data: r.data.to_vec(),
            done: r.done,
        };

        let resp = RAFT.install_snapshot(rpc).await.or_else(
            |err|  Err(Status::invalid_argument("Raft Error"))
        )?;
        Ok(Response::new(InstallSnapshotResp {
            term: resp.term
        }))
    }

    async fn vote(&self, request: Request<VoteReq>) -> Result<Response<VoteResp>, Status> {
        let r = request.get_ref();
        let rpc = VoteRequest {
            term: r.term,
            candidate_id: r.candidate_id,
            last_log_index: r.last_log_index,
            last_log_term: r.last_log_term
        };

        let resp = RAFT.vote(rpc).await.or_else(
            |err|  Err(Status::invalid_argument("Raft Error"))
        )?;

        Ok(Response::new(VoteResp {
            term: resp.term,
            vote_granted: resp.vote_granted
        }))
    }
}


pub async fn start_rpc_server(addr: &str) -> Result<()> {
    let addr = addr.parse()?;
    let router = RPC;
    Server::builder()
        .add_service(RpcServiceServer::new(router))
        .serve(addr)
        .await?;
    Ok(())
}