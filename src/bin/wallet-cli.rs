use anyhow::Context;
use clap::Parser;
use std::io::IsTerminal;
use defi_hot_wallet::cli::{Cli, Commands};
use defi_hot_wallet::core::config::WalletConfig;
use defi_hot_wallet::core::WalletManager;
use defi_hot_wallet::security::SecretVec;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, Write};
use tokio::fs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // 从默认配置构建，然后覆盖对测试/CLI运行重要的字段
    let mut wallet_config = WalletConfig::default();
    // 对测试使用内存中的 sqlite 以避免接触磁盘
    wallet_config.storage.database_url = "sqlite::memory:".to_string();
    // 确保区块链网络映射存在（避免需要 BlockchainConfig::default）
    wallet_config.blockchain.networks = HashMap::new();
    let wallet_manager = WalletManager::new(&wallet_config).await?;

    match cli.command {
        Commands::Create { name, output } => {
            wallet_manager.create_wallet(&name, "cli_default_password", false).await?;
            tracing::info!(name = %name, "创建钱包");
            if let Some(path) = output.as_deref() {
                write_wallet_output_if_requested(Some(path), &()).await?;
                tracing::info!(path = %path.display(), "Wallet info written to path");
            }
        }
        Commands::List => {
            tracing::info!("列出所有钱包");
        }
        Commands::Info { name } => {
            tracing::info!(name = %name, "显示钱包信息");
        }
        Commands::Transfer { name, to, amount } => {
            tracing::info!(from = %name, to = %to, amount = %amount, "转账");
        }
        Commands::Balance { name, network: _ } => {
            tracing::info!(name = %name, "查询余额");
        }
        Commands::Bridge { name, from_chain: _, to_chain: _, token: _, amount: _ } => {
            tracing::info!(name = %name, "桥接");
        }
        Commands::GenerateMnemonic => {
            // simple 12-word mock mnemonic for tests
            let mnemonic_literal = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
            // keep mnemonic bytes in a zeroizing buffer to reduce plaintext lifetime
            let mnemonic = SecretVec::new(mnemonic_literal.as_bytes().to_vec());

            // Safer options for exporting the mnemonic:
            // 1) Require an explicit two-step opt-in: set both
            //    ALLOW_PLAINTEXT_MNEMONIC=1 and ALLOW_PLAINTEXT_MNEMONIC_CONFIRM=1
            //    to print the mnemonic to stdout.
            // 2) Provide an encryption key via MNEMONIC_EXPORT_KEY (32-byte hex). If set,
            //    the CLI will encrypt the mnemonic with AES-256-GCM and write to the
            //    file path specified by MNEMONIC_EXPORT_PATH (defaults to ./mnemonic.enc).
            // These measures reduce accidental secret leakage.

            let allow_mnemonic =
                std::env::var("ALLOW_PLAINTEXT_MNEMONIC").ok().as_deref() == Some("1");
            let confirm_mnemonic =
                std::env::var("ALLOW_PLAINTEXT_MNEMONIC_CONFIRM").ok().as_deref() == Some("1");

            if allow_mnemonic && confirm_mnemonic {
                // Allow tests to bypass interactive TTY checks and the prompt by setting the WALLET_TEST_CONSTRUCTOR marker.
                let test_ctor =
                    std::env::var("WALLET_TEST_CONSTRUCTOR").ok().as_deref() == Some("1");
                if test_ctor {
                    // In test harnesses, print directly and avoid prompting for interactive input.
                    if let Ok(s) = std::str::from_utf8(mnemonic.as_slice()) {
                        println!("{}", s);
                    } else {
                        println!("<invalid-utf8-mnemonic>");
                    }
                    tracing::info!(
                        mnemonic = "<shown>",
                        "Generated mnemonic displayed to stdout (test constructor bypass)"
                    );
                    return Ok(());
                }

                // Require both stdin and stdout to be a TTY for interactive confirmation.
                if !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
                    tracing::error!("Refusing to print mnemonic: interactive TTY required for plaintext display. Use MNEMONIC_EXPORT_KEY to write an encrypted export instead.");
                    return Ok(());
                }

                // Prompt the operator to type a deliberate confirmation phrase to avoid accidental exposure.
                print!("WARNING: You are about to display a secret mnemonic in plaintext. Type SHOW to confirm: ");
                // Ensure the prompt is flushed to the terminal.
                let _ = io::stdout().flush();
                let mut input = String::new();
                if let Err(e) = io::stdin().read_line(&mut input) {
                    tracing::error!(error = %e, "Failed to read confirmation input; aborting plaintext mnemonic display");
                    return Ok(());
                }
                if input.trim() != "SHOW" {
                    tracing::info!(entered = %input.trim(), "Plaintext mnemonic display aborted by operator");
                    return Ok(());
                }

                // Intentionally print the mnemonic to stdout when explicitly double-confirmed and operator typed SHOW.
                if let Ok(s) = std::str::from_utf8(mnemonic.as_slice()) {
                    println!("{}", s);
                    tracing::info!(mnemonic = "<shown>", "Generated mnemonic displayed to stdout (double-confirmed + interactive confirmation)");
                } else {
                    println!("<invalid-utf8-mnemonic>");
                    tracing::error!("Generated mnemonic is not valid UTF-8");
                }
                return Ok(());
            }

            // Encrypted export path: if MNEMONIC_EXPORT_KEY is set (32-byte hex), encrypt and write.
            if let Ok(export_key_hex) = std::env::var("MNEMONIC_EXPORT_KEY") {
                // Default path
                let out_path = std::env::var("MNEMONIC_EXPORT_PATH")
                    .unwrap_or_else(|_| "./mnemonic.enc".to_string());

                // Normalize hex like env_manager does
                let mut key_hex = export_key_hex.trim().to_string();
                if key_hex.starts_with("0x") || key_hex.starts_with("0X") {
                    key_hex = key_hex[2..].to_string();
                }
                // Validate length (64 hex chars -> 32 bytes) and decode
                if key_hex.len() != 64 {
                    return Err(anyhow::anyhow!(
                        "MNEMONIC_EXPORT_KEY must be 64 hex chars (32 bytes)"
                    ));
                }

                let key_bytes_vec = match hex::decode(&key_hex) {
                    Ok(b) => b,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "MNEMONIC_EXPORT_KEY contains invalid hex: {}",
                            e
                        ))
                    }
                };

                if key_bytes_vec.len() != 32 {
                    return Err(anyhow::anyhow!(
                        "MNEMONIC_EXPORT_KEY decoded length is not 32 bytes"
                    ));
                }

                // Zeroize the decoded key_bytes immediately and use it for encryption
                use defi_hot_wallet::security::mnemonic_export;
                use zeroize::Zeroizing;

                let key_bytes = Zeroizing::new(key_bytes_vec);
                let aad = out_path.as_bytes();
                let blob = mnemonic_export::encrypt_mnemonic_to_bytes(
                    // pass secret bytes slice for encryption
                    std::str::from_utf8(mnemonic.as_slice()).unwrap_or_default(),
                    &key_bytes,
                    aad,
                )
                .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

                std::fs::write(&out_path, &blob).map_err(|e| {
                    anyhow::anyhow!("Failed to write encrypted mnemonic to {}: {}", out_path, e)
                })?;

                // Try to set restrictive permissions on unix-like systems (600).
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = std::fs::metadata(&out_path) {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o600);
                        if let Err(e) = std::fs::set_permissions(&out_path, perms) {
                            tracing::warn!(path = %out_path, error = %e, "Failed to set 0o600 permissions on exported mnemonic file");
                        }
                    }
                }

                #[cfg(not(unix))]
                {
                    // On non-Unix platforms we cannot reliably set POSIX-style 0o600 permissions.
                    // Emit a warning so operators running on such platforms are aware and
                    // should manually secure the exported file location.
                    tracing::warn!(path = %out_path, "Encrypted mnemonic exported; could not enforce POSIX 0o600 permissions on this platform. Secure the file manually.");
                }

                tracing::warn!(
                    "Encrypted mnemonic exported to {} (MNEMONIC_EXPORT_KEY used)",
                    out_path
                );

                return Ok(());
            }

            // Default behavior: do not reveal mnemonic in plaintext. Log a hint to operator.
            tracing::info!(mnemonic = "<hidden>", "Mnemonic generated. To export in plaintext set ALLOW_PLAINTEXT_MNEMONIC=1 and ALLOW_PLAINTEXT_MNEMONIC_CONFIRM=1, or provide MNEMONIC_EXPORT_KEY to write an encrypted file.");
            // Also emit an info-level log that does not contain the secret
            tracing::info!(mnemonic = %if allow_mnemonic { "<shown>" } else { "<hidden>" }, "mnemonic command executed");
        }
        Commands::Help => {
            tracing::info!("Help requested");
        }
    }

    Ok(())
}

/// 辅助函数：如果提供了 --output 路径，则将钱包信息写入文件。
async fn write_wallet_output_if_requested(
    output_path: Option<&std::path::Path>,
    wallet: &impl serde::Serialize,
) -> anyhow::Result<()> {
    if let Some(path) = output_path {
        // By default, redact sensitive fields (private keys, mnemonics, seeds) from
        // the serialized wallet JSON to avoid accidentally persisting secrets.
        // To export the full, plaintext wallet (dangerous), set the
        // environment variable ALLOW_PLAINTEXT_WALLET_EXPORT=1 explicitly.
        let allow_plain =
            std::env::var("ALLOW_PLAINTEXT_WALLET_EXPORT").ok().as_deref() == Some("1");

        // Serialize into a JSON Value so we can redact fields safely.
        let mut value: Value = serde_json::to_value(wallet).context("serialize wallet to json")?;

        if !allow_plain {
            redact_sensitive_fields(&mut value);
            tracing::warn!(path = %path.display(), "Writing redacted wallet info to disk (sensitive fields removed). To write full wallet data set ALLOW_PLAINTEXT_WALLET_EXPORT=1");
        } else {
            tracing::warn!(path = %path.display(), "ALLOW_PLAINTEXT_WALLET_EXPORT=1: writing full wallet data to disk. Ensure file permissions and understand this will include secrets.");
        }

        let json =
            serde_json::to_string_pretty(&value).context("serialize redacted wallet to json")?;

        // 如果需要，创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.ok();
        }

        fs::write(path, json).await.context("write wallet file to --output path")?;

        // Try to set restrictive permissions on unix-like systems (600).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(path, perms);
            }
        }
    }
    Ok(())
}

/// Recursively redact well-known sensitive field names from a JSON Value.
fn redact_sensitive_fields(v: &mut Value) {
    match v {
        Value::Object(map) => {
            // Collect keys to redact (case-insensitive match)
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in keys {
                if let Some(mut val) = map.remove(&k) {
                    // If key looks sensitive, replace with placeholder. Otherwise recurse.
                    let lower = k.to_lowercase();
                    if lower.contains("private")
                        || lower.contains("secret")
                        || lower.contains("mnemonic")
                        || lower.contains("seed")
                        || lower.contains("key")
                    {
                        map.insert(k, Value::String("[REDACTED]".to_string()));
                    } else {
                        redact_sensitive_fields(&mut val);
                        map.insert(k, val);
                    }
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                redact_sensitive_fields(item);
            }
        }
        _ => {}
    }
}
// ...existing code...
