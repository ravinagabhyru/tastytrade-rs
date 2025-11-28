use tastytrade_rs::api::order::Symbol;
use tastytrade_rs::api::oauth2::OAuth2Config;
use tastytrade_rs::TastyTrade;

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
    use super::{OAuth2Config, TastyTrade};

    /// Get OAuth2 config from environment variables
    /// Returns None if not available (tests should be skipped)
    #[allow(dead_code)]
    pub fn oauth_config() -> Option<(OAuth2Config, String)> {
        let client_id = env::var("TT_OAUTH_CLIENT_ID").ok()?;
        let client_secret = env::var("TT_OAUTH_CLIENT_SECRET").ok()?;
        let redirect_uri = env::var("TT_OAUTH_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost".to_string());
        let refresh_token = env::var("TT_OAUTH_REFRESH_TOKEN").ok()?;

        let config = OAuth2Config {
            client_id,
            client_secret,
            redirect_uri,
            scopes: vec!["read".to_string()],
        };
        Some((config, refresh_token))
    }

    /// Create a TastyTrade client using OAuth env vars (demo environment)
    #[allow(dead_code)]
    pub async fn create_demo_client() -> Option<TastyTrade> {
        let (config, refresh_token) = oauth_config()?;
        TastyTrade::from_refresh_token(config, &refresh_token, true).await.ok()
    }

    /// Get demo account number from environment
    #[allow(dead_code)] // Used in integration tests
    pub fn demo_account_number() -> Option<String> {
        env::var("TEST_ACCOUNT_NUMBER").ok()
    }
}

/// Async test utilities
#[allow(dead_code)]
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
