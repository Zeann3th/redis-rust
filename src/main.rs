use std::{
    io::{BufReader, Read},
    net::TcpListener,
    sync::{Arc, Mutex},
};

use crate::{common::Environment, resp2::serialization::Deserialize};

mod common;
mod resp2;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    let env = Arc::new(Mutex::new(Environment::new()));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let env_clone = Arc::clone(&env);

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
                        resp2.set_stream(stream.try_clone().unwrap());
                        resp2.set_environment(env_clone.clone());

                        if let Err(e) = resp2.deserialize(buf[..n].to_vec()) {
                            println!("Failed to deserialize RESP2: {}", e);
                            break;
                        }

                        if let Err(e) = resp2.reflect() {
                            println!("Error handling command: {}", e);
                            break;
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
