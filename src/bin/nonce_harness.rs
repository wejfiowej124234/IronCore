use std::env;
use std::time::Duration;
use tokio::time::sleep;

use defi_hot_wallet::security::redaction::redact_body;
use defi_hot_wallet::storage::{WalletStorage, WalletStorageTrait};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        tracing::error!("usage: nonce_harness <db_path> <network> <address> <count>");
        std::process::exit(2);
    }
    let db_path = &args[1];
    let network = &args[2];
    let address = &args[3];
    let count: usize = args[4].parse()?;

    let db_url = if db_path.starts_with("sqlite:") {
        db_path.to_string()
    } else {
        // Use an absolute path and normalize separators so sqlite driver can open it on Windows
        let abs =
            std::fs::canonicalize(db_path).unwrap_or_else(|_| std::path::PathBuf::from(db_path));
        let mut abs_s = abs.to_string_lossy().replace("\\", "/");
        // Remove Windows extended path prefix if present (e.g. //?/C:/...)
        if abs_s.starts_with("//?/") {
            abs_s = abs_s.trim_start_matches("//?/").to_string();
        }
        // If path looks like /C:/... (leading slash before drive) strip it
        if abs_s.starts_with('/') && abs_s.len() > 2 && abs_s.as_bytes()[2] == b':' {
            abs_s = abs_s.trim_start_matches('/').to_string();
        }
        // Use triple-slash prefix for absolute paths on Windows
        format!("sqlite:///{}", abs_s)
    };

    let storage: WalletStorage = match WalletStorage::new_with_url(&db_url).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Error initializing storage: {}", redact_body(&e.to_string()));
            std::process::exit(1);
        }
    };

    for _ in 0..count {
        let n = storage.reserve_next_nonce(network, address, 0).await?;
        // Print only the numeric nonce to stdout for callers — not sensitive
        println!("{}", n);
        // small jitter
        sleep(Duration::from_millis(10)).await;
    }

    Ok(())
}
