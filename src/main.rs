use std::{
    io::{BufReader, Read},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
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

    let env = Arc::new(Mutex::new(Environment::new(role.clone(), port)));

    if role == "slave" {
        let mut master_stream = TcpStream::connect(host).expect("Failed to connect to master");

        let mut init = Resp2::new(env.clone());
        init.set_kind(resp2::command::RespCommand::INTITIALIZE);
        if let Err(e) = init.reflect(&mut master_stream) {
            eprintln!("Failed to initialize slave: {}", e);
            return;
        }

        spawn_master_listener(master_stream, Arc::clone(&env));
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let env_clone = Arc::clone(&env);
                thread::spawn(move || handle_client(stream, env_clone));
            }
            Err(e) => {
                println!("Connection failed: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream, env: Arc<Mutex<Environment>>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut buffer = Vec::new();

    loop {
        let mut temp_buf = [0u8; 1024];
        let n = match reader.get_mut().read(&mut temp_buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                println!("Failed to read from stream: {}", e);
                break;
            }
        };

        buffer.extend_from_slice(&temp_buf[..n]);

        while let Some((command_bytes, used)) = match try_parse_one_command(&buffer) {
            Ok(result) => result,
            Err(e) => {
                println!("Parse error: {}", e);
                return;
            }
        } {
            let mut resp2 = Resp2::new(env.clone());
            resp2.set_literal(command_bytes.clone());

            if let Err(e) = resp2.deserialize(command_bytes.clone()) {
                println!("Failed to deserialize: {}", e);
                return;
            }

            if let Err(e) = resp2.reflect(&mut stream) {
                println!("Command error: {}", e);
                return;
            }

            buffer.drain(..used);
        }
    }
}

fn spawn_master_listener(stream: TcpStream, env: Arc<Mutex<Environment>>) {
    thread::spawn(move || handle_client(stream, env));
}

fn try_parse_one_command(buf: &[u8]) -> Result<Option<(Vec<u8>, usize)>, String> {
    if buf.is_empty() {
        return Ok(None);
    }

    let input = match std::str::from_utf8(buf) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    let mut lines = input.split("\r\n").peekable();
    let mut total = 0;

    let first = match lines.next() {
        Some(line) => line,
        None => return Ok(None),
    };
    total += first.len() + 2;

    if !first.starts_with('*') {
        return Err(format!("Expected RESP array, got '{}'", first));
    }

    let count = match first[1..].parse::<usize>() {
        Ok(n) => n,
        Err(_) => return Err(format!("Invalid array count '{}'", first)),
    };

    for _ in 0..count {
        let size_line = lines.next().ok_or("Missing $length line")?;
        total += size_line.len() + 2;
        if !size_line.starts_with('$') {
            return Err(format!("Expected bulk string, got '{}'", size_line));
        }

        let len = size_line[1..]
            .parse::<usize>()
            .map_err(|_| format!("Invalid bulk length '{}'", size_line))?;

        let data_line = lines.next().ok_or("Missing bulk string value")?;
        total += data_line.len() + 2;

        if data_line.len() != len {
            return Err("Bulk string length mismatch".to_string());
        }
    }

    if total > buf.len() {
        return Ok(None);
    }

    Ok(Some((buf[..total].to_vec(), total)))
}
