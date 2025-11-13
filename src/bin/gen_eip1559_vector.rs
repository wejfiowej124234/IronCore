use clap::Parser;
use defi_hot_wallet::security::redaction::redact_body;
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Eip1559TransactionRequest, NameOrAddress, U256};

#[derive(Parser)]
struct Args {
    /// Print raw signed hex to stdout (default for generator)
    #[arg(long, default_value_t = false)]
    raw: bool,
}

fn main() {
    let args = Args::parse();
    // deterministic private key (32 bytes)
    let priv_key =
        hex::decode("0101010101010101010101010101010101010101010101010101010101010101").unwrap();
    let wallet = LocalWallet::from_bytes(&priv_key).expect("wallet").with_chain_id(1u64);

    let to = NameOrAddress::Address("0x1111111111111111111111111111111111111111".parse().unwrap());
    let tx_req = Eip1559TransactionRequest {
        to: Some(to),
        value: Some(U256::from(1_000_000_000_000_000u64)),
        gas: Some(U256::from(21000u64)),
        max_fee_per_gas: Some(U256::from(20_000_000_000u64)),
        max_priority_fee_per_gas: Some(U256::from(1_000_000_000u64)),
        nonce: Some(U256::from(0u64)),
        ..Default::default()
    };

    let typed: ethers::types::transaction::eip2718::TypedTransaction = tx_req.into();
    let sig = futures::executor::block_on(wallet.sign_transaction(&typed)).expect("sign");
    let signed_bytes = typed.rlp_signed(&sig).to_vec();

    // Ensure vectors directory exists
    tracing_subscriber::fmt::init();
    let out_dir = std::path::Path::new("./vectors");
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        tracing::error!("failed to create vectors dir: {}", redact_body(&e.to_string()));
        std::process::exit(1);
    }

    // Prepare JSON vector metadata. Include signed hex only when --raw is passed.
    let mut vector = serde_json::json!({
        "name": "eip1559_example",
        "chain_id": 1,
        "to": "0x1111111111111111111111111111111111111111",
        "value": "1000000000000000",
        "gas": 21000,
        "max_fee_per_gas": "20000000000",
        "max_priority_fee_per_gas": "1000000000",
        "nonce": 0u64,
    });

    if args.raw {
        vector["signed_tx_hex"] =
            serde_json::Value::String(format!("0x{}", hex::encode(&signed_bytes)));
    }

    let out_path = out_dir.join("eip1559_vector.json");
    if let Err(e) = std::fs::write(&out_path, serde_json::to_string_pretty(&vector).unwrap()) {
        tracing::error!("failed to write vector file: {}", redact_body(&e.to_string()));
        std::process::exit(1);
    }

    // Try to restrict file permissions to owner read/write where supported.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(err) =
            std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o600))
        {
            tracing::warn!(
                "failed to set secure permissions on {}: {}",
                out_path.display(),
                redact_body(&err.to_string())
            );
        }
    }

    // Structured log: don't print secret hex — print redacted summary and filename
    tracing::info!(
        "Generated EIP-1559 vector written: {} include_hex={}",
        out_path.display(),
        args.raw
    );
}
