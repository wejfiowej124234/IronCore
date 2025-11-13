//! tests/hello_world_tests.rs
//!
//! Basic hello world tests for demonstration.

#[cfg(test)]
mod tests {
    #[test]
    fn test_hello_world() {
        assert_eq!("Hello, World!", "Hello, World!");
    }

    #[test]
    fn test_addition() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_string_length() {
        let s = "Hello, World!";
        assert_eq!(s.len(), 13);
    }

    #[test]
    fn test_vector_operations() {
        let mut v = vec![1, 2, 3];
        v.push(4);
        assert_eq!(v, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_option_some() {
        let opt = Some(42);
        assert_eq!(opt, Some(42));
    }
}
