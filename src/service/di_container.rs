#[derive(Default)]
pub struct DiContainer;
impl DiContainer {
    pub fn new() -> Self {
        Self
    }
}

// The user requested to add `impl Default`, but `#[derive(Default)]` is more idiomatic.
// If a manual implementation is needed, it would be:
// impl Default for DiContainer { fn default() -> Self { Self::new() } }
