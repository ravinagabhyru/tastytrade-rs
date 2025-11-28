use chrono::{DateTime, Duration, Utc};

use crate::api::oauth2::OAuth2Config;

/// OAuth2 authentication state
#[derive(Debug, Clone)]
pub struct AuthState {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub config: OAuth2Config,
}

impl AuthState {
    /// Generate Authorization header value (Bearer token)
    pub fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    /// Whether the access token should be refreshed.
    /// Applies a 60-second buffer before expiry.
    pub fn needs_refresh(&self) -> bool {
        if let Some(exp) = self.expires_at {
            let refresh_threshold = exp - Duration::seconds(60);
            Utc::now() >= refresh_threshold
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_header() {
        let cfg = OAuth2Config {
            client_id: "id".into(),
            client_secret: "sec".into(),
            redirect_uri: "http://localhost".into(),
            scopes: vec!["read".into()],
        };
        let state = AuthState {
            access_token: "token".into(),
            refresh_token: Some("refresh".into()),
            expires_at: None,
            config: cfg,
        };
        assert_eq!(state.auth_header(), "Bearer token");
    }

    #[test]
    fn test_needs_refresh_thresholds() {
        let cfg = OAuth2Config {
            client_id: "id".into(),
            client_secret: "sec".into(),
            redirect_uri: "http://localhost".into(),
            scopes: vec![],
        };
        let now = Utc::now();

        // Far from expiry -> false
        let state_far = AuthState {
            access_token: "t".into(),
            refresh_token: Some("r".into()),
            expires_at: Some(now + Duration::seconds(300)),
            config: cfg.clone(),
        };
        assert!(!state_far.needs_refresh());

        // Within 60 seconds -> true
        let state_soon = AuthState {
            access_token: "t".into(),
            refresh_token: Some("r".into()),
            expires_at: Some(now + Duration::seconds(30)),
            config: cfg.clone(),
        };
        assert!(state_soon.needs_refresh());

        // Past expiry -> true
        let state_past = AuthState {
            access_token: "t".into(),
            refresh_token: Some("r".into()),
            expires_at: Some(now - Duration::seconds(1)),
            config: cfg,
        };
        assert!(state_past.needs_refresh());

        // No expiry set -> false
        let state_no_exp = AuthState {
            access_token: "t".into(),
            refresh_token: None,
            expires_at: None,
            config: OAuth2Config {
                client_id: "id".into(),
                client_secret: "sec".into(),
                redirect_uri: "http://localhost".into(),
                scopes: vec![],
            },
        };
        assert!(!state_no_exp.needs_refresh());
    }
}

