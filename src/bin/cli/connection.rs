use std::io::prelude::*;
use std::io::{BufReader, ErrorKind, Result};
use std::net::{TcpStream, ToSocketAddrs};

use super::{Decoder, Value};

pub struct Connection {
    stream: BufReader<TcpStream>,
    decoder: Decoder,
}

impl Connection {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let tcp = TcpStream::connect(addr)?;
        Ok(Connection {
            stream: BufReader::new(tcp),
            decoder: Decoder::new(),
        })
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<()> {
        let stream = self.stream.get_mut() as &mut dyn Write;
        stream.write_all(buf)
    }

    pub fn read(&mut self) -> Result<Value> {
        if let Some(value) = self.decoder.read() {
            return Ok(value);
        }
        loop {
            let consumed_len = {
                let buffer = match self.stream.fill_buf() {
                    Ok(buf) => buf,
                    Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
                    Err(err) => return Err(err),
                };

                if buffer.len() == 0 {
                    continue;
                }
                self.decoder.feed(&buffer)?;
                buffer.len()
            };

            self.stream.consume(consumed_len);
            if let Some(value) = self.decoder.read() {
                return Ok(value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{encode_slice, Value};
    use super::*;

    #[test]
    fn struct_connection() {
        let mut connection = Connection::new("127.0.0.1:6379").unwrap();
        connection
            .write(&encode_slice(&["set", "rust", "test_redis_cli"]))
            .unwrap();
        assert_eq!(connection.read().unwrap(), Value::String("OK".to_string()));

        connection.write(&encode_slice(&["get", "rust"])).unwrap();
        assert_eq!(
            connection.read().unwrap(),
            Value::Bulk("test_redis_cli".to_string())
        );

        connection
            .write(&encode_slice(&["set", "rust", "test_redis_cli_2"]))
            .unwrap();
        connection.write(&encode_slice(&["get", "rust"])).unwrap();
        assert_eq!(connection.read().unwrap(), Value::String("OK".to_string()));
        assert_eq!(
            connection.read().unwrap(),
            Value::Bulk("test_redis_cli_2".to_string())
        );
    }
}
