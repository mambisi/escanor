use async_raft::{RaftStorage, NodeId, RaftMetrics};
use crate::codec::{ClientRequest, ServerResponse};
use async_raft::raft::{Entry, MembershipConfig, EntryPayload};
use async_raft::storage::{HardState, InitialState, CurrentSnapshotData};
use anyhow::Result;
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite};
use async_trait::async_trait;
use serde_json::Value;
use sled::{Db, IVec, Error, Tree};
use tokio::fs::File;
use tokio::sync::RwLock;
use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::io::Cursor;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use itertools::Itertools;
use bincode::ErrorKind;
use rayon::prelude::*;
use crate::{db, EscanorRaft};
use crate::command;
use redis_protocol::types::{Frame, RedisProtocolError};
use redis_protocol;
use bytes::{BytesMut, BufMut};
use nom::AsBytes;
use futures::Future;
use byteorder::{ByteOrder, BigEndian};
use tracing::Instrument;
use tracing::{debug, error, info, span, warn, Level};
use std::collections::HashSet;

use crate::file_dirs::create_db_folder;
use crate::command::{AddClusterCmd, RemClusterCmd, ClusterCmd, ClusterMetrics, ClusterSetNodeId};
use crate::network::Context;
use crate::printer::{print_err, print_ok, print_arr, print_str};
use tokio::runtime::Runtime;

const NODE_TREE_KEY: &str = "cluster_nodes";
const CLUSTER_NODE_ID_KEY: &str = "cluster_node_id";
const CLUSTER_METRICS_KEY: &str = "cluster_metrics";
use crate::RAFT;

lazy_static!(
  static ref SYS_STATE : Arc<Db> = {
        let config = sled::Config::new().mode(sled::Mode::HighThroughput).path(create_db_folder("sys"));
        let db = config.open().expect("failed to open database");
        return Arc::new(db);
  };
);

pub fn init() {
    lazy_static::initialize(&SYS_STATE);
}

pub async fn monitor_metrics() {

    tokio::task::spawn(async {
        let state = SYS_STATE.clone();
        loop {
            match RAFT.metrics().recv().await {
                Some(metrics) => {
                    match  serde_json::to_string(&metrics) {
                        Ok(json) => {
                            state.insert(CLUSTER_METRICS_KEY, IVec::from(json.as_bytes()));
                        }
                        Err(_) => {}
                    };
                }
                _ => {}
            }
        }
    });
}


pub fn get_sys_state() -> Arc<Db> {
    SYS_STATE.clone()
}


pub fn add_cluster(_: Arc<std::sync::RwLock<Context>>, cmd: &AddClusterCmd) -> String {
    let nodes_tree = match SYS_STATE.open_tree(NODE_TREE_KEY) {
        Ok(tree) => {
            tree
        }
        Err(_) => {
            return print_err("internal error");
        }
    };
    let mut buff = [0; 16];
    BigEndian::write_u64(&mut buff, cmd.arg_node_id);
    nodes_tree.insert(buff, cmd.arg_addrs.as_bytes());
    RAFT.add_non_voter(cmd.arg_node_id);
    print_ok()
}

pub fn rem_cluster(_: Arc<std::sync::RwLock<Context>>, cmd: &RemClusterCmd) -> String {
    let nodes_tree = match SYS_STATE.open_tree(NODE_TREE_KEY) {
        Ok(tree) => {
            tree
        }
        Err(_) => {
            return print_err("internal error");
        }
    };
    let mut buff = [0; 16];
    BigEndian::write_u64(&mut buff, cmd.arg_node_id);
    nodes_tree.remove(&buff);
    RAFT.add_non_voter(cmd.arg_node_id);

    let members: HashSet<NodeId> = nodes_tree.iter().map(|r| {
        let (k, _) = r.unwrap();
        let node_id: NodeId = BigEndian::read_u64(&k);
        node_id
    }).collect();
    RAFT.change_membership(members);
    print_ok()
}

pub fn cluster(_: Arc<std::sync::RwLock<Context>>, cmd: &ClusterCmd) -> String {
    let nodes_tree = match SYS_STATE.open_tree(NODE_TREE_KEY) {
        Ok(tree) => {
            tree
        }
        Err(_) => {
            return print_err("internal error");
        }
    };

    let members: Vec<String> = nodes_tree.iter().map(|r| {
        let (k, v) = r.unwrap();
        let node_id = BigEndian::read_u64(&k);
        let addrs = String::from_utf8(v.to_vec()).unwrap_or("NULL".to_owned());
        format!("[{}] {}", node_id, addrs)
    }).collect();
    print_arr(members)
}

pub fn cluster_metrics(_: Arc<std::sync::RwLock<Context>>, cmd: &ClusterMetrics) -> String {
    let json = match SYS_STATE.get(CLUSTER_METRICS_KEY) {
        Ok(r) => {
            match r {
                None => {
                    return print_str("nil")
                }
                Some(v) => {
                    String::from_utf8(v.to_vec()).unwrap_or("nil".to_owned())
                }
            }
        }
        Err(_) => {
            return print_str("nil")
        }
    };
    print_str(&json)
}

pub fn cluster_set_node_id(_: Arc<std::sync::RwLock<Context>>, cmd: &ClusterSetNodeId) -> String {
    let mut buff = [0; 16];
    BigEndian::write_u64(&mut buff, cmd.arg_node_id);
    SYS_STATE.insert(&NODE_TREE_KEY, &buff);
    print_ok()
}

pub fn get_node_id() -> NodeId {
    match SYS_STATE.get(&NODE_TREE_KEY){
        Ok(r) => {
            match r {
                None => {
                    2002
                }
                Some(v) => {
                    let node_id: NodeId = BigEndian::read_u64(&v);
                    node_id
                }
            }
        }
        Err(_) => {
            2002
        }
    }
}

pub fn get_cluster_members() -> HashSet<NodeId> {
    let nodes_tree = match SYS_STATE.open_tree(NODE_TREE_KEY) {
        Ok(tree) => {
            tree
        }
        Err(_) => {
            return HashSet::new();
        }
    };
    let members: HashSet<NodeId> = nodes_tree.iter().map(|r| {
        let (k, _) = r.unwrap();
        let node_id: NodeId = BigEndian::read_u64(&k);
        node_id
    }).collect();
    members
}

pub struct Storage {
    id: NodeId,
    sys: Arc<sled::Db>,
    log: Arc<sled::Db>,
    hs: RwLock<Option<HardState>>,
    current_snapshot: RwLock<Option<StorageSnapshot>>,
}

const LAST_APPLIED_LOG_KEY: &str = "last_applied_log";
const ERR_INCONSISTENT_LOG: &str = "a query was received which was expecting data to be in place which does not exist in the log";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageSnapshot {
    /// The last index covered by this snapshot.
    pub index: u64,
    /// The term of the last index covered by this snapshot.
    pub term: u64,
    /// The last memberhsip config included in this snapshot.
    pub membership: MembershipConfig,
    /// The data of the state machine at the time of this snapshot.
    pub data: Vec<u8>,
}

impl Storage {
    pub fn new(id: NodeId) -> Self {

        let log = sled::open(create_db_folder("log")).expect("failed to initialize storage");
        let sys = get_sys_state();
        sys.insert(LAST_APPLIED_LOG_KEY, "0");
        return Storage {
            id,
            sys,
            log: Arc::new(log),
            hs: RwLock::new(None),
            current_snapshot: RwLock::new(None),
        };
    }
}


#[async_trait]
impl RaftStorage<ClientRequest, ServerResponse> for Storage {
    type Snapshot = Cursor<Vec<u8>>;

    #[tracing::instrument(level = "trace", skip(self))]
    async fn get_membership_config(&self) -> Result<MembershipConfig> {
        let cfg_opt = self.log.iter().rev().find_map(|entry| {
            let (_, v) = entry.unwrap();
            let entry: Entry<ClientRequest> = bincode::deserialize(&v).unwrap();
            match &entry.payload {
                EntryPayload::ConfigChange(cfg) => Some(cfg.membership.clone()),
                EntryPayload::SnapshotPointer(snap) => Some(snap.membership.clone()),
                _ => None,
            }
        });

        Ok(match cfg_opt {
            Some(cfg) => cfg,
            None => MembershipConfig::new_initial(self.id),
        })
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn get_initial_state(&self) -> Result<InitialState> {
        let membership = self.get_membership_config().await?;
        let mut hs = self.hs.write().await;
        return match &mut *hs {
            None => {
                let new = InitialState::new_initial(self.id);
                *hs = Some(new.hard_state.clone());
                Ok(new)
            }
            Some(inner) => {
                let (last_log_index, last_log_term) = match self.log.iter().rev().next() {
                    Some(entry) => {
                        let (_, v) = entry.unwrap();
                        let entry: Entry<ClientRequest> = bincode::deserialize(&v).unwrap();
                        (entry.index, entry.term)
                    }
                    None => (0, 0),
                };

                Ok(InitialState {
                    last_log_index,
                    last_log_term,
                    last_applied_log: 0,
                    hard_state: inner.clone(),
                    membership,
                })
            }
        };
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn save_hard_state(&self, hs: &HardState) -> Result<()> {
        *self.hs.write().await = Some(hs.clone());
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn get_log_entries(&self, start: u64, stop: u64) -> Result<Vec<Entry<ClientRequest>>> {
        if start > stop {
            return Ok(vec![]);
        }
        let b = start.to_be_bytes();
        let t = stop.to_be_bytes();
        Ok(self.log.range(b..t).map(|res| {
            let (_, value) = res.unwrap();
            let entry: Entry<ClientRequest> = bincode::deserialize(&value).unwrap();
            entry
        }).collect())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn delete_logs_from(&self, start: u64, stop: Option<u64>) -> Result<()> {
        if stop.as_ref().map(|stop| &start > stop).unwrap_or(false) {
            return Ok(());
        }
        // If a stop point was specified, delete from start until the given stop point.
        if let Some(stop) = stop.as_ref() {
            for key in start..*stop {
                self.log.remove(key.to_be_bytes());
            }
            return Ok(());
        }

        let items: Vec<IVec> = self.log.get_lt(start.to_be_bytes()).iter().map(|r| {
            let r = r.clone();
            let (k, _) = r.unwrap();
            k
        }).collect();

        let mut batch = sled::Batch::default();
        for k in items {
            batch.remove(k)
        }
        self.log.apply_batch(batch);
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn append_entry_to_log(&self, entry: &Entry<ClientRequest>) -> Result<()> {
        let entry_bytes = bincode::serialize(entry)?;
        self.log.insert(entry.index.to_be_bytes(), entry_bytes)?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn replicate_to_log(&self, entries: &[Entry<ClientRequest>]) -> Result<()> {
        let mut batch = sled::Batch::default();
        for entry in entries {
            let entry_bytes = bincode::serialize(entry).unwrap();
            batch.insert(IVec::from(&entry.index.to_be_bytes()), IVec::from(entry_bytes));
        }
        self.log.apply_batch(batch);
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn apply_entry_to_state_machine(&self, index: &u64, data: &ClientRequest) -> Result<ServerResponse> {
        let context = data.context.clone();
        let frame = &data.frame;

        let response_message = match command::compile_frame(frame) {
            Ok(cmd) => {
                let res = cmd.execute(context).to_owned();
                res
            }
            Err(err) => {
                return Ok(ServerResponse {
                    frame: Frame::Error(err.to_string())
                });
            }
        };
        let mut buff = [0; 16];
        BigEndian::write_u64(&mut buff, *index);
        self.sys.insert(LAST_APPLIED_LOG_KEY, IVec::from(&buff));
        let buf: BytesMut = BytesMut::from(response_message.as_bytes());
        let (frame, _) = redis_protocol::decode::decode_bytes(&buf).unwrap();

        match frame {
            None => {
                Ok(ServerResponse {
                    frame: Frame::Null
                })
            }
            Some(frame) => {
                Ok(ServerResponse {
                    frame
                })
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn replicate_to_state_machine(&self, entries: &[(&u64, &ClientRequest)]) -> Result<()> {
        for (index, entry) in entries {
            let frame = &entry.frame;
            let context = entry.context.clone();
            let cmd = command::compile_frame(frame)?;
            cmd.execute(context);
            let mut buff = [0; 16];
            BigEndian::write_u64(&mut buff, **index);
            self.sys.insert(LAST_APPLIED_LOG_KEY, IVec::from(&buff));
        }
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn do_log_compaction(&self, through: u64) -> Result<CurrentSnapshotData<Self::Snapshot>> {
        let (data, last_applied_log);
        {
            // Serialize the data of the state machine.
            data = db::export_db()?;

            let d = match self.sys.get(LAST_APPLIED_LOG_KEY)? {
                None => {
                    let zero: u64 = 0;
                    IVec::from(&zero.to_be_bytes())
                }
                Some(s) => {
                    s
                }
            };

            last_applied_log = BigEndian::read_u64(&d)
        } // Release state machine read lock.

        let membership_config;
        {
            // Go backwards through the log to find the most recent membership config <= the `through` index.
            membership_config = self.log.iter().rev().find_map(|entry| {
                let (_, v) = entry.unwrap();
                let entry: Entry<ClientRequest> = bincode::deserialize(&v).unwrap();
                match &entry.payload {
                    EntryPayload::ConfigChange(cfg) => Some(cfg.membership.clone()),
                    _ => None,
                }
            }).unwrap_or(MembershipConfig::new_initial(self.id));
        } // Release log read lock.

        let snapshot_bytes: Vec<u8>;
        let term;
        {
            let mut current_snapshot = self.current_snapshot.write().await;

            term = self.log.get(last_applied_log.to_be_bytes()).map(|entry| {
                let v = entry.unwrap();
                let entry: Entry<ClientRequest> = bincode::deserialize(&v).unwrap();
                entry.term
            }).or_else(|_| Err(anyhow::anyhow!(ERR_INCONSISTENT_LOG)))?;


            let items: Vec<IVec> = self.log.get_lt(last_applied_log.to_be_bytes()).iter().map(|r| {
                let r = r.clone();
                let (k, _) = r.unwrap();
                k
            }).collect();

            let mut batch = sled::Batch::default();
            for k in items {
                batch.remove(k)
            }
            self.log.apply_batch(batch);

            let e: Entry<ClientRequest> = Entry::new_snapshot_pointer(last_applied_log, term, "".into(), membership_config.clone());
            let e_to_vec = bincode::serialize(&e)?;
            self.log.insert(
                last_applied_log.to_string(),
                e_to_vec,
            );

            let snapshot = StorageSnapshot {
                index: last_applied_log,
                term,
                membership: membership_config.clone(),
                data,
            };

            snapshot_bytes = bincode::serialize(&snapshot)?;
            *current_snapshot = Some(snapshot);
        }

        Ok(CurrentSnapshotData {
            term,
            index: last_applied_log,
            membership: membership_config.clone(),
            snapshot: Box::new(Cursor::new(snapshot_bytes)),
        })
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn create_snapshot(&self) -> Result<(String, Box<Self::Snapshot>)> {
        Ok((String::from(""), Box::new(Cursor::new(Vec::new()))))
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn finalize_snapshot_installation(&self, index: u64, term: u64, delete_through: Option<u64>, id: String, snapshot: Box<Self::Snapshot>) -> Result<()> {
        info!("{:#?}", snapshot);
        let membership_config = self.log.iter().rev().find_map(|entry| {
            let (_, v) = entry.unwrap();
            let entry: Entry<ClientRequest> = bincode::deserialize(&v).unwrap();
            match &entry.payload {
                EntryPayload::ConfigChange(cfg) => Some(cfg.membership.clone()),
                _ => None,
            }
        }).unwrap_or(MembershipConfig::new_initial(self.id));
        match &delete_through {
            Some(through) => {
                let items: Vec<IVec> = self.log.get_lt(through.to_be_bytes()).iter().map(|r| {
                    let r = r.clone();
                    let (k, _) = r.unwrap();
                    k
                }).collect();

                let mut batch = sled::Batch::default();
                for k in items {
                    batch.remove(k)
                }
                self.log.apply_batch(batch);
            }
            None => {
                self.log.clear();
            }
        }
        let e: Entry<ClientRequest> = Entry::new_snapshot_pointer(index, term, id, membership_config);
        let e_to_vec = bincode::serialize(&e)?;
        self.log.insert(IVec::from(&index.to_be_bytes()), e_to_vec);
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn get_current_snapshot(&self) -> Result<Option<CurrentSnapshotData<Self::Snapshot>>> {
        match &*self.current_snapshot.read().await {
            None => Ok(None),
            Some(snapshot) => {
                let reader = bincode::serialize(snapshot)?;
                Ok(Some(CurrentSnapshotData {
                    index: snapshot.index,
                    term: snapshot.term,
                    membership: snapshot.membership.clone(),
                    snapshot: Box::new(Cursor::new(reader)),
                }))
            }
        }
    }
}