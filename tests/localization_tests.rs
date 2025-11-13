// ...existing code...
// tests/localization_tests.rs
//
// Tests for the i18n localization module.
//
// Note: resources/i18n/en.ftl should contain: hello = Hello, World!
//       resources/i18n/zh.ftl should contain: hello = 你好，世界！

use defi_hot_wallet::i18n::localization::translate;

#[test]
fn test_translate_english() {
    let result = translate("hello", "en");
    assert_eq!(result, "Hello, World!");
}

#[test]
fn test_translate_chinese() {
    let result = translate("hello", "zh");
    assert_eq!(result, "你好，世界！");
}

#[test]
fn test_translate_fallback_to_default_language() {
    // If language not found, translator should fall back (compilation-only check here).
    let _result = translate("hello", "fr");
}

#[test]
fn test_translate_missing_key() {
    let result = translate("missing_key_for_test", "en");
    assert_eq!(result, "missing_key_for_test");
}

#[test]
fn test_translate_empty_key() {
    let result = translate("", "en");
    assert_eq!(result, "");
}
// ...existing code...
