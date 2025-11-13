//! Defines the main application entry point.

// ...existing code...
//! Application-level container for services.

use crate::service::WalletService;

/// The main application struct, holding the service registry.
#[derive(Debug, Default)]
pub struct Application {
    services: WalletService,
}

impl Application {
    /// Create a new `Application` using `Default` for contained services.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an `Application` with an injected `WalletService`.
    pub fn with_service(services: WalletService) -> Self {
        Self { services }
    }

    /// Shared (immutable) access to the wallet service registry.
    pub fn services(&self) -> &WalletService {
        &self.services
    }

    /// Mutable access to the wallet service registry.
    pub fn services_mut(&mut self) -> &mut WalletService {
        &mut self.services
    }
}
// ...existing code...
