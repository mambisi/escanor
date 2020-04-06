use console::style;

extern crate tokio;

use tokio::net::TcpListener;
use tokio::prelude::*;
use std::sync::Arc;
use std::sync::RwLock;

use lazy_static::lazy_static;


use crate::{command, db};
use std::fmt::Debug;
use std::collections::BTreeMap;

use crate::printer;
use crate::printer::{print_err, print_from_error};

const MB: usize = 1_048_576;

pub async fn start_up(addr: &str) -> Result<(), Box<dyn std::error::Error>> {

    let mut listener = TcpListener::bind(addr).await?;

    println!("{} started tcp server on {}", style("Escanor").red(), addr);

    loop {
        let (mut socket, _) = listener.accept().await?;

        println!("received packet size:{}", socket.recv_buffer_size().unwrap_or(0));

        tokio::spawn(async move {
            let mut buf = vec![0; 2 * MB].into_boxed_slice();
            // In a loop, read data from the socket and write the data back.
            loop {
                let n = match socket.read(&mut buf).await {
                    // socket closed
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                let cmd_string = match String::from_utf8(buf[0..n].to_vec()) {
                    Ok(s) => s,
                    Err(e) => {
                        println!("{}", e);
                        return;
                    }
                };

                let message: String = match command::parse(&cmd_string) {
                    Ok(cmd) => {
                       let res = cmd.execute().to_owned();
                       res
                    }
                    Err(e) => {
                        println!("{}", e);
                        print_from_error(&e)
                    }
                };
                // Write the data back
                if let Err(e) = socket.write_all(message.as_bytes()).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}