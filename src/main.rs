use std::{
    io::{BufReader, Read, Write},
    net::TcpListener,
};

use crate::resp2::{command::Resp2Command, serialization::Deserialize};

mod resp2;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());

                    loop {
                        let mut buf = vec![0; 1024];
                        let n = match reader.get_mut().read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => n,
                            Err(e) => {
                                println!("Failed to read from stream: {}", e);
                                break;
                            }
                        };

                        let mut resp2 = resp2::Resp2::new();
                        match resp2.deserialize(buf[..n].to_vec()) {
                            Ok(_) => match resp2.kind {
                                Resp2Command::PING => {
                                    if let Err(e) = reader.get_mut().write_all(b"+PONG\r\n") {
                                        println!("Failed to write to stream: {}", e);
                                        break;
                                    }
                                }
                                _ => {
                                    println!("Received command: {}", resp2.kind);
                                }
                            },
                            Err(e) => {
                                println!("Failed to deserialize RESP2: {}", e);
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                println!("Connection failed: {}", e);
            }
        }
    }
}
