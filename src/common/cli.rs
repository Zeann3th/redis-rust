use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "redis")]
#[command(version = "0.1.0")]
#[command(about = "Mini Redis Clone", long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value_t = 6379)]
    pub port: u16,

    #[command(subcommand)]
    pub command: Option<CLICommand>,
}

#[derive(Subcommand)]
pub enum CLICommand {
    Info {
        #[arg(default_value = "default")]
        section: String,
    },
}
