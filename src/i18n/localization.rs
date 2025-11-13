/// 根据给定的 key 和语言码返回本地化文本。
///
/// # Arguments
/// * `key` - 文本键
/// * `lang` - 语言码（例如 "en", "zh"）
///
/// # Returns
/// 对应的本地化字符串；若找不到对应文本则返回原始 key。
pub fn translate(key: &str, lang: &str) -> String {
    match (lang, key) {
        ("en", "hello") => "Hello, World!".to_string(),
        ("zh", "hello") => "你好，世界！".to_string(),

        ("en", "wallet-create") => "Create Wallet".to_string(),
        ("zh", "wallet-create") => "创建钱包".to_string(),

        // 对于其他语言，使用英文作为默认替代文本
        (_, "hello") => "Hello, World!".to_string(),
        (_, "wallet-create") => "Create Wallet".to_string(),

        // 默认：返回 key 本身
        (_, k) => k.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_english() {
        assert_eq!(translate("wallet-create", "en"), "Create Wallet");
    }

    #[test]
    fn test_translate_chinese() {
        assert_eq!(translate("wallet-create", "zh"), "创建钱包");
    }

    #[test]
    fn test_translate_fallback() {
        // 未知语言应回退到英文替代文本
        assert_eq!(translate("wallet-create", "fr"), "Create Wallet");
    }

    #[test]
    fn test_translate_missing_key() {
        // 缺失 key 时返回原始 key
        assert_eq!(translate("missing_key_for_test", "en"), "missing_key_for_test");
    }
}
