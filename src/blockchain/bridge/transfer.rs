// filepath: src/blockchain/bridge/transfer.rs
use crate::blockchain::traits::Bridge;
use crate::core::wallet_info::SecureWalletData;
use tracing::info;

pub async fn initiate_bridge_transfer(
    bridge: &dyn Bridge,
    from_chain: &str,
    to_chain: &str,
    token: &str,
    amount: &str,
    wallet_data: &SecureWalletData,
) -> anyhow::Result<String> {
    // SECURITY: Validate all input parameters to prevent injection attacks
    validate_bridge_parameters(from_chain, to_chain, token, amount)?;

    info!(
        "Initiating bridge transfer of {} {} from {} to {} via bridge",
        amount, token, from_chain, to_chain
    );
    bridge.transfer_across_chains(from_chain, to_chain, token, amount, wallet_data).await
}

/// Validate bridge transfer parameters to prevent injection and invalid input attacks
fn validate_bridge_parameters(
    from_chain: &str,
    to_chain: &str,
    token: &str,
    amount: &str,
) -> anyhow::Result<()> {
    // Validate chain names - only allow alphanumeric characters and hyphens
    if !is_valid_chain_name(from_chain) {
        return Err(anyhow::anyhow!("Invalid from_chain name: {}", from_chain));
    }
    if !is_valid_chain_name(to_chain) {
        return Err(anyhow::anyhow!("Invalid to_chain name: {}", to_chain));
    }

    // Prevent same-chain transfers
    if from_chain == to_chain {
        return Err(anyhow::anyhow!("Cannot bridge to the same chain"));
    }

    // Validate token symbol - only allow alphanumeric characters
    if !token.chars().all(|c| c.is_alphanumeric()) || token.is_empty() || token.len() > 20 {
        return Err(anyhow::anyhow!("Invalid token symbol: {}", token));
    }

    // Validate amount format and range
    validate_bridge_amount(amount)?;

    Ok(())
}

/// Validate chain name format
fn is_valid_chain_name(chain: &str) -> bool {
    !chain.is_empty()
        && chain.len() <= 50
        && chain.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Validate bridge amount
fn validate_bridge_amount(amount: &str) -> anyhow::Result<()> {
    if amount.is_empty() {
        return Err(anyhow::anyhow!("Amount cannot be empty"));
    }

    // Strict decimal validation: no floats/exponents; up to 18 decimals; > 0
    crate::core::validation::validate_amount_strict(amount, 18)?;

    Ok(())
}
