#[cfg(test)]
mod tests {
    use defi_hot_wallet::mvp::*;

    #[test]
    fn test_bridge_assets_amount_none() {
        let result = bridge_assets_amount(None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid amount");
    }

    #[test]
    fn test_bridge_assets_amount_empty() {
        let result = bridge_assets_amount(Some(""));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid amount");
    }

    #[test]
    fn test_bridge_assets_amount_invalid() {
        let result = bridge_assets_amount(Some("!@#$%^&*()"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid amount");
    }

    #[test]
    fn test_send_transaction_empty_wallet() {
        let result = send_transaction("", Some(100)); // 淇锛氭坊鍔?Some
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid wallet name");
    }

    #[test]
    fn test_send_transaction_none_amount() {
        let result = send_transaction("valid_wallet", None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid amount");
    }

    #[test]
    fn test_send_transaction_invalid_wallet() {
        let result = send_transaction("!@#$%^&*()", Some(100)); // 淇锛氭坊鍔?Some
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid wallet name");
    }

    #[test]
    fn test_create_wallet_empty() {
        let result = create_wallet("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid wallet name");
    }

    #[test]
    fn test_create_wallet_invalid() {
        let result = create_wallet("!@#$%^&*()");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid wallet name");
    }

    #[test]
    fn test_calculate_bridge_fee_none() {
        let result = calculate_bridge_fee(None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid amount");
    }

    #[test]
    fn test_calculate_bridge_fee_invalid() {
        let result = calculate_bridge_fee(Some("!@#$%^&*()"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid amount");
    }
}
