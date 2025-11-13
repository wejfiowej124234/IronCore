use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// DeFi Hot Wallet CLI (library-facing definitions)
#[derive(Debug, Parser)]
#[command(name = "wallet-cli", about = "DeFi Hot Wallet CLI", disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Create {
        /// Wallet name
        #[arg(long)]
        name: String,
        /// Optional output path
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Info {
        #[arg(long)]
        name: String,
    },
    Transfer {
        #[arg(long)]
        name: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: String,
    },
    Balance {
        #[arg(long)]
        name: String,
        #[arg(long)]
        network: Option<String>,
    },
    Bridge {
        #[arg(long = "name")]
        name: String,
        #[arg(long = "from-chain")]
        from_chain: String,
        #[arg(long = "to-chain")]
        to_chain: String,
        #[arg(long)]
        token: String,
        #[arg(long)]
        amount: String,
    },
    List,
    GenerateMnemonic,
    Help,
}
