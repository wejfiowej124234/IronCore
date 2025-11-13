#[cfg(test)]
mod tests {
    #[test]
    fn test_hello_world() {
        assert_eq!("hello world", "hello world");
    }

    #[tokio::test]
    async fn test_async_hello_world() {
        // Simple async test that always passes
        let result = tokio::spawn(async { "hello async world" }).await.unwrap();

        assert_eq!(result, "hello async world");
    }
}
