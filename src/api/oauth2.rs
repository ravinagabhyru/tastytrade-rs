use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::api::base::Result;

/// OAuth2 client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

/// OAuth2 token response from Tastytrade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
}

/// Serializable OAuth2 token for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Token {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub obtained_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
}

impl OAuth2Token {
    /// Create a token from API response and set obtained_at to now
    pub fn from_response(response: OAuth2TokenResponse) -> Self {
        Self {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            token_type: response.token_type,
            expires_in: response.expires_in,
            obtained_at: Utc::now(),
            id_token: response.id_token,
        }
    }

    /// Check if token is expired (with a 60s buffer)
    pub fn is_expired(&self) -> bool {
        let expires_at = self.obtained_at + Duration::seconds(self.expires_in);
        Utc::now() >= (expires_at - Duration::seconds(60))
    }

    /// Compute absolute expiration time
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.obtained_at + Duration::seconds(self.expires_in)
    }

    /// Serialize token to JSON string
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize token from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

/// OAuth2 authorization/refresh request payload
#[derive(Debug, Serialize)]
pub struct OAuth2AuthRequest {
    pub grant_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub client_id: String,
    pub client_secret: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
}

/// Builder for OAuth2 client configuration and convenience constructors
pub struct OAuth2ClientBuilder {
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: Option<String>,
    scopes: Vec<String>,
    demo: bool,
}

impl OAuth2ClientBuilder {
    pub fn new() -> Self {
        Self {
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            scopes: vec![],
            demo: false,
        }
    }

    pub fn client_id(mut self, id: impl Into<String>) -> Self {
        self.client_id = Some(id.into());
        self
    }

    pub fn client_secret(mut self, secret: impl Into<String>) -> Self {
        self.client_secret = Some(secret.into());
        self
    }

    pub fn redirect_uri(mut self, uri: impl Into<String>) -> Self {
        self.redirect_uri = Some(uri.into());
        self
    }

    pub fn add_scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    pub fn demo(mut self, demo: bool) -> Self {
        self.demo = demo;
        self
    }

    pub fn build(self) -> Result<OAuth2Config> {
        let client_id = self
            .client_id
            .ok_or_else(|| crate::api::base::TastyError::Config("client_id is required".into()))?;
        let client_secret = self.client_secret.ok_or_else(|| {
            crate::api::base::TastyError::Config("client_secret is required".into())
        })?;
        let redirect_uri = self.redirect_uri.ok_or_else(|| {
            crate::api::base::TastyError::Config("redirect_uri is required".into())
        })?;

        Ok(OAuth2Config {
            client_id,
            client_secret,
            redirect_uri,
            scopes: self.scopes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_roundtrip() {
        let t = OAuth2Token {
            access_token: "a".into(),
            refresh_token: "r".into(),
            token_type: "Bearer".into(),
            expires_in: 1200,
            obtained_at: Utc::now(),
            id_token: None,
        };

        let s = t.to_json().unwrap();
        let t2 = OAuth2Token::from_json(&s).unwrap();
        assert_eq!(t2.access_token, t.access_token);
        assert_eq!(t2.refresh_token, t.refresh_token);
        assert_eq!(t2.expires_in, t.expires_in);
    }

    #[test]
    fn test_expiry_math() {
        let mut t = OAuth2Token {
            access_token: "a".into(),
            refresh_token: "r".into(),
            token_type: "Bearer".into(),
            expires_in: 120,
            obtained_at: Utc::now() - Duration::seconds(30),
            id_token: None,
        };
        // 90s remaining, buffer 60 => not expired
        assert!(!t.is_expired());

        // Shift obtained_at to simulate near expiry (50s remaining)
        t.obtained_at = Utc::now() - Duration::seconds(70);
        assert!(t.is_expired());
    }

    #[test]
    fn test_builder_validation() {
        let err = OAuth2ClientBuilder::new().build().unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("client_id is required"));
    }
}

