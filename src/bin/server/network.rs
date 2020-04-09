use console::style;

extern crate tokio;

use tokio::net::TcpListener;
use tokio::prelude::*;
use crate::command;
use crate::db;
use std::fmt::Debug;
use std::collections::BTreeMap;

use crate::printer;
use crate::printer::{print_err, print_from_error};

use std::time::Duration;
use std::sync::Arc;

const MB: usize = 1_048_576;

pub async fn start_up(addr: &str) -> Result<(), Box<dyn std::error::Error>> {

    let mut listener = TcpListener::bind(addr).await?;

    println!("{} started tcp server on {}", style("Escanor").red(), addr);

    let mut counter = 0_u64;

    loop {
        let (mut socket, _) = listener.accept().await?;
        //let thread_pool = Builder::new().max_threads(1).build();
        counter = counter + 1;

        print!("Socket Connections {}",counter);

        tokio::spawn(async move {
            let mut buf = vec![0; 3000].into_boxed_slice();
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
}
use std::net::TcpListener as TcpL;
use std::thread::spawn;
use tungstenite::server::accept;
use tungstenite::Message;

pub fn start_up_ws(addr : &str) {
    let server = TcpL::bind(addr).unwrap();
    for stream in server.incoming() {
        spawn (move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                let msg = websocket.read_message().unwrap();

                let message: String = match command::parse(&msg.into_data()) {
                    Ok(cmd) => {
                        let res = cmd.execute().to_owned();
                        res
                    }
                    Err(e) => {
                        print_from_error(&e)
                    }
                };

                websocket.write_message(Message::Text(message));
            }
        });
    }
}