use std::fmt::Display;

pub enum Resp2Command {
    PING,
    UNDEFINED,
    PONG
}

impl Resp2Command {
    pub fn from_str(cmd: &str) -> Self {
        match cmd.to_uppercase().as_str() {
            "PING" => Resp2Command::PING,
            "PONG" => Resp2Command::PONG,
            _ => Resp2Command::UNDEFINED,
        }
    }
}

impl Display for Resp2Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resp2Command::PING => write!(f, "PING"),
            Resp2Command::UNDEFINED => write!(f, "UNDEFINED"),
            Resp2Command::PONG => write!(f, "PONG"),
        }
    }
}
