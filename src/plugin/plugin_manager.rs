use crate::plugin::plugin::Plugin;

#[derive(Default)]
pub struct PluginManager;
impl PluginManager {
    pub fn new() -> Self {
        Self
    }
    pub fn register(&self, _p: Box<dyn Plugin>) {}
}

// The user requested to add `impl Default`, but `#[derive(Default)]` is more idiomatic.
// If a manual implementation is needed, it would be:
// impl Default for PluginManager { fn default() -> Self { Self::new() } }
