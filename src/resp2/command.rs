use std::fmt::Display;

pub enum Command {
    PING,
    UNDEFINED,
    PONG,
    ECHO,
    SET,
    GET,
}

impl Command {
    pub fn from_str(cmd: &str) -> Self {
        match cmd.to_uppercase().as_str() {
            "PING" => Command::PING,
            "PONG" => Command::PONG,
            "ECHO" => Command::ECHO,
            "SET" => Command::SET,
            "GET" => Command::GET,
            _ => Command::UNDEFINED,
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::PING => write!(f, "PING"),
            Command::UNDEFINED => write!(f, "UNDEFINED"),
            Command::PONG => write!(f, "PONG"),
            Command::ECHO => write!(f, "ECHO"),
            Command::SET => write!(f, "SET"),
            Command::GET => write!(f, "GET"),
        }
    }
}
