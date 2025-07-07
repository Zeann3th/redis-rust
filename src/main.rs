use std::{
    io::{BufReader, Read},
    net::TcpListener,
    sync::{Arc, Mutex},
};

use crate::{
    common::Environment,
    resp2::{serialization::Deserialize, Resp2},
};

mod common;
mod resp2;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut port: u16 = 6379;
    let mut role = "master".to_string();
    let mut host = (String::new(), 0u16);

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("Usage: redis [options]");
                println!("Options:");
                println!("  -h, --help        Show this help message");
                println!("  -v, --version     Show version information");
                println!("  -p, --port <PORT> Specify the port to listen on (default: 6379)");
                return;
            }
            "-v" | "--version" => {
                println!("Redis server version 0.1.0");
                return;
            }
            "-p" | "--port" => {
                if i + 1 >= args.len() {
                    eprintln!("Expected port number after '{}'", args[i]);
                    return;
                }
                match args[i + 1].parse::<u16>() {
                    Ok(p) => port = p,
                    Err(_) => {
                        eprintln!("Invalid port: '{}'", args[i + 1]);
                        return;
                    }
                }
                i += 1;
            }
            "--replicaof" => {
                if i + 1 >= args.len() {
                    eprintln!("Expected host and port after '{}'", args[i]);
                    return;
                }

                let replicaof = &args[i + 1];
                host = replicaof
                    .split_once(' ')
                    .map(|(h, p)| (h.to_string(), p.parse::<u16>().unwrap_or(0)))
                    .unwrap_or((String::new(), 0));

                let (master_host, master_port) = host.clone();

                if master_host.is_empty() || master_port == 0 {
                    eprintln!("Invalid host or port for --replicaof: '{}'", replicaof);
                    return;
                }

                role = "slave".to_string();
                i += if args[i + 1].contains(' ') { 1 } else { 2 };
            }
            unknown => {
                eprintln!("Unknown argument '{}'. Use -h for help.", unknown);
                return;
            }
        }
        i += 1;
    }

    let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
    println!("Listening on 127.0.0.1:{}", port);

    let env = Arc::new(Mutex::new(Environment::new(
        role.clone(),
        port,
        Some(host.clone()),
        "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".to_string(),
        0,
    )));

    if role.clone() == "slave" {
        let mut resp2 = Resp2::new(env.clone());
        resp2.set_kind(resp2::command::Resp2Command::INTITIALIZE);
        match resp2.reflect() {
            Ok(_) => println!("Slave initialized successfully."),
            Err(e) => {
                eprintln!("Failed to initialize slave: {}", e);
                return;
            }
        };
    }

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

                        let mut resp2 = resp2::Resp2::new(env_clone.clone());
                        resp2.set_stream(stream.try_clone().unwrap());

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
