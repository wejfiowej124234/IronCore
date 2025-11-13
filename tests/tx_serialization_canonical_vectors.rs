// Canonical transaction serialization and signing vectors for regression tests.
// This test uses the deterministic private key and tx parameters from the generator binaries
// and asserts the signed raw hex matches the canonical vector. This protects against
// accidental changes to signing/serialization (EIP-1559 and legacy + EIP-155 replay protection).

use ethers::signers::{LocalWallet, Signer};
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Eip1559TransactionRequest, NameOrAddress, TransactionRequest, U256};

#[tokio::test]
async fn canonical_eip1559_vector() {
    // Deterministic private key (matches gen_eip1559_vector)
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

    let typed: TypedTransaction = tx_req.into();
    let sig = wallet.sign_transaction(&typed).await.expect("sign");
    let signed_bytes = typed.rlp_signed(&sig).to_vec();

    let canonical = "0x02f8728080843b9aca008504a817c80082520894111111111111111111111111111111111111111187038d7ea4c6800080c001a0691191bf06d248f41d30c17e4712b95e4b01998fdd97ec3b02784d7214002f7aa068c0c986842abcc69f3f79ea622e6c2e632b3a5ac90b6269f36a6bf7dcb92951";
    let hexed = format!("0x{}", hex::encode(&signed_bytes));
    assert_eq!(canonical.to_lowercase(), hexed.to_lowercase());
}

#[tokio::test]
async fn canonical_legacy_eip155_vector() {
    // Deterministic private key (matches gen_legacy_eip155_vector)
    let priv_key =
        hex::decode("0101010101010101010101010101010101010101010101010101010101010101").unwrap();
    let wallet = LocalWallet::from_bytes(&priv_key).expect("wallet").with_chain_id(1u64);

    let to = NameOrAddress::Address("0x1111111111111111111111111111111111111111".parse().unwrap());
    let tx_req = TransactionRequest {
        to: Some(to),
        value: Some(U256::from(1_000_000_000_000_000u64)),
        gas: Some(U256::from(21000u64)),
        gas_price: Some(U256::from(20_000_000_000u64)),
        nonce: Some(U256::from(0u64)),
        ..Default::default()
    };

    let typed: TypedTransaction = tx_req.into();
    let sig = wallet.sign_transaction(&typed).await.expect("sign");
    let signed_bytes = typed.rlp_signed(&sig).to_vec();

    let canonical = "0xf86b808504a817c80082520894111111111111111111111111111111111111111187038d7ea4c680008025a0bf102036c4c1d09b980801dd71748a55c56b9df26e9c0d4c95cdf3e33a27147fa075907e1ef77b1e1bfb8b708b671729fb41c5b4031ef1baca3a9ff70931d2880c";
    let hexed = format!("0x{}", hex::encode(&signed_bytes));
    assert_eq!(canonical.to_lowercase(), hexed.to_lowercase());
}
