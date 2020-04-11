use console::style;

extern crate tokio;

use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use crate::command;
use crate::db;
use std::fmt::Debug;
use std::collections::BTreeMap;

use crate::printer;
use crate::printer::{print_err, print_from_error};

use std::time::Duration;
use std::sync::Arc;
use std::net::SocketAddr;

const MB: usize = 1_048_576;

fn process_socket(mut socket: TcpStream) {
    // do work with socket here
    tokio::spawn(async move {
        let mut buf = vec![0; 10 * MB].into_boxed_slice();
        // In a loop, read data from the socket and write the data back.
        loop {
            let n = match socket.read(&mut buf).await {
                // socket closed
                Ok(n) if n == 0 => return,
                Ok(n) => n,
                Err(e) => {
                    return;
                }
            };

            let message: String = match command::parse(&buf[0..n]) {
                Ok(cmd) => {
                    let res = cmd.execute().to_owned();
                    res
                }
                Err(e) => {
                    print_from_error(&e)
                }
            };
            // Write the data back
            if let Err(e) = socket.write_all(message.as_bytes()).await {
                return;
            }
        }
    });
}


pub async fn start_up(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind(addr).await?;

    printer::print_app_info();

    info!("{}", style("Server initialized").green());

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                process_socket(socket);
            }
            Err(e) => error!("couldn't get client: {:?}", e),
        };
    }
}