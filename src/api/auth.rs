use chrono::{DateTime, Duration, Utc};

use crate::api::oauth2::OAuth2Config;

#[derive(Debug, Clone)]
pub enum AuthMode {
    /// Legacy session-based authentication using /sessions
    Session { session_token: String },
    /// OAuth2-based authentication using access and refresh tokens
    OAuth2 {
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>, // absolute expiry of access token
        config: OAuth2Config,
    },
}

impl AuthMode {
    /// Generate Authorization header value for the current auth mode
    pub fn auth_header(&self) -> String {
        match self {
            AuthMode::Session { session_token } => session_token.clone(),
            AuthMode::OAuth2 { access_token, .. } => format!("Bearer {}", access_token),
        }
    }

    /// Whether the current OAuth2 access token should be refreshed
    /// Returns false for Session auth. Applies a 60-second buffer before expiry.
    pub fn needs_refresh(&self) -> bool {
        match self {
            AuthMode::Session { .. } => false,
            AuthMode::OAuth2 { expires_at, .. } => {
                if let Some(exp) = expires_at {
                    // refresh if within 60 seconds of expiry (or past expiry)
                    let refresh_threshold = *exp - Duration::seconds(60);
                    Utc::now() >= refresh_threshold
                } else {
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_header_session() {
        let mode = AuthMode::Session {
            session_token: "abc".to_string(),
        };
        assert_eq!(mode.auth_header(), "abc");
    }

    #[test]
    fn test_auth_header_oauth2() {
        let cfg = OAuth2Config {
            client_id: "id".into(),
            client_secret: "sec".into(),
            redirect_uri: "http://localhost".into(),
            scopes: vec!["read".into()],
        };
        let mode = AuthMode::OAuth2 {
            access_token: "token".into(),
            refresh_token: Some("refresh".into()),
            expires_at: None,
            config: cfg,
        };
        assert_eq!(mode.auth_header(), "Bearer token");
    }

    #[test]
    fn test_needs_refresh_false_for_session() {
        let mode = AuthMode::Session {
            session_token: "abc".to_string(),
        };
        assert!(!mode.needs_refresh());
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
        let mode_far = AuthMode::OAuth2 {
            access_token: "t".into(),
            refresh_token: Some("r".into()),
            expires_at: Some(now + Duration::seconds(300)),
            config: cfg.clone(),
        };
        assert_eq!(mode_far.needs_refresh(), false);

        // Within 60 seconds -> true
        let mode_soon = AuthMode::OAuth2 {
            access_token: "t".into(),
            refresh_token: Some("r".into()),
            expires_at: Some(now + Duration::seconds(30)),
            config: cfg.clone(),
        };
        assert_eq!(mode_soon.needs_refresh(), true);

        // Past expiry -> true
        let mode_past = AuthMode::OAuth2 {
            access_token: "t".into(),
            refresh_token: Some("r".into()),
            expires_at: Some(now - Duration::seconds(1)),
            config: cfg,
        };
        assert_eq!(mode_past.needs_refresh(), true);
    }
}

