use tastytrade_rs::api::order::Symbol;

/// Common test symbols used across tests
pub mod test_symbols {
    use super::Symbol;

    #[allow(dead_code)] // Used in integration tests
    pub fn aapl() -> Symbol {
        Symbol::from("AAPL")
    }
    #[allow(dead_code)] // Used in integration tests
    pub fn spy() -> Symbol {
        Symbol::from("SPY")
    }
    #[allow(dead_code)] // Used in integration tests
    pub fn msft() -> Symbol {
        Symbol::from("MSFT")
    }
}

/// Environment variable helpers for integration tests
pub mod env {
    use std::env;

    /// Get demo credentials from environment variables
    /// Returns None if not available (tests should be skipped)
    pub fn demo_credentials() -> Option<(String, String)> {
        let username = env::var("DEMO_USERNAME").ok()?;
        let password = env::var("DEMO_PASSWORD").ok()?;
        Some((username, password))
    }

    /// Get demo account number from environment
    #[allow(dead_code)] // Used in integration tests
    pub fn demo_account_number() -> Option<String> {
        env::var("TEST_ACCOUNT_NUMBER").ok()
    }
}

/// Async test utilities
pub mod async_utils {
    use std::time::Duration;

    /// Create a test runtime with timeout for async tests
    pub async fn with_timeout<F, T>(
        duration: Duration,
        future: F,
    ) -> Result<T, tokio::time::error::Elapsed>
    where
        F: std::future::Future<Output = T>,
    {
        tokio::time::timeout(duration, future).await
    }

    /// Long timeout for integration tests (30 seconds)
    pub const INTEGRATION_TIMEOUT: Duration = Duration::from_secs(30);
}
