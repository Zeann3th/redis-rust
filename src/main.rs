use std::{
    io::{BufReader, Read},
    net::TcpListener,
    sync::{Arc, Mutex},
};

use crate::{common::Environment, resp2::serialization::Deserialize};

mod common;
mod resp2;

fn main() {
    let mut listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    let args: Vec<String> = std::env::args().collect();

    match args.len() {
        1 => {}
        2 => match args[1].as_str() {
            "--help" | "-h" => {
                println!("Usage: redis [options] [port]");
                println!("Options:");
                println!("  --help, -h     Show this help message");
                println!("  --version, -v  Show version information");
                println!("  --port, -p     Specify the port to listen on (default: 6379)");
            }
            "--version" | "-v" => {
                println!("Redis server version 0.1.0");
            }
            _ => {
                println!(
                    "Unknown option: {}. Use '-h' to find out more options",
                    args[1]
                );
            }
        },
        3 => match args[1].as_str() {
            "--port" | "-p" => {
                let port = match args[2].parse::<u16>() {
                    Ok(p) => p,
                    Err(_) => {
                        println!("Invalid port number: {}", args[2]);
                        return;
                    }
                };
                listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
            }
            _ => {
                println!(
                    "Unknown option: {}. Use '-h' to find out more options",
                    args[1]
                );
                return;
            }
        },
        _ => {
            println!("Too many arguments. Use '-h' to find out more options");
            return;
        }
    };

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
