use std::io::{Error, ErrorKind, Result};
use std::net::ToSocketAddrs;

use super::connection::Connection;
use super::{encode_slice, Value};

pub fn create_client(hostname: &str, port: u16, password: &str, db: u16) -> Result<Client> {
    let mut client = Client::new((hostname, port))?;
    client.init(password, db)?;
    Ok(client)
}

pub struct Client {
    conn: Connection,
}

impl Client {
    pub fn new<A: ToSocketAddrs>(addrs: A) -> Result<Self> {
        Ok(Client {
            conn: Connection::new(addrs)?,
        })
    }

    pub fn cmd(&mut self, slice: &[&str]) -> Result<Value> {
        let buf = encode_slice(slice);
        match self.conn.write(&buf){
            Ok(_) => {},
            Err(_) => {},
        };
        self.conn.read()
    }

    pub fn read_more(&mut self) -> Result<Value> {
        self.conn.read()
    }

    fn init(&mut self, password: &str, db: u16) -> Result<()> {
        if password.len() > 0 {
            if let Value::Error(err) = self.cmd(&["auth", password])? {
                return Err(Error::new(ErrorKind::PermissionDenied, err));
            }
        }
        if db > 0 {
            if let Value::Error(err) = self.cmd(&["select", &db.to_string()])? {
                return Err(Error::new(ErrorKind::InvalidInput, err));
            }
        }
        Ok(())
    }
}
