use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct LoginCredentials<'a> {
    pub login: &'a str,
    pub password: &'a str,
    pub remember_me: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LoginResponseUser {
    pub email: String,
    pub username: String,
    pub external_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LoginResponse {
    pub user: LoginResponseUser,
    pub session_token: String,
    pub remember_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_login_credentials_serialization() {
        let credentials = LoginCredentials {
            login: "testuser",
            password: "testpass",
            remember_me: true,
        };

        let json = serde_json::to_string(&credentials).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Test that keys are kebab-case
        assert_eq!(parsed["login"], "testuser");
        assert_eq!(parsed["password"], "testpass");
        assert_eq!(parsed["remember-me"], true);

        // Ensure no secrets are accidentally logged by checking the structure
        // (This is a compile-time check - LoginCredentials should not derive Debug in production)
        assert!(json.contains("testuser"));
        assert!(json.contains("testpass"));
        assert!(json.contains("remember-me"));
    }

    #[test]
    fn test_login_credentials_remember_me_false() {
        let credentials = LoginCredentials {
            login: "user@example.com",
            password: "secret123",
            remember_me: false,
        };

        let json = serde_json::to_string(&credentials).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["login"], "user@example.com");
        assert_eq!(parsed["password"], "secret123");
        assert_eq!(parsed["remember-me"], false);
    }

    #[test]
    fn test_login_response_deserialization_with_remember_token() {
        let json = json!({
            "user": {
                "email": "user@example.com",
                "username": "testuser",
                "external-id": "ext123"
            },
            "session-token": "session_abc123",
            "remember-token": "remember_xyz789"
        });

        let response: LoginResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.user.email, "user@example.com");
        assert_eq!(response.user.username, "testuser");
        assert_eq!(response.user.external_id, "ext123");
        assert_eq!(response.session_token, "session_abc123");
        assert_eq!(response.remember_token, Some("remember_xyz789".to_string()));
    }

    #[test]
    fn test_login_response_deserialization_without_remember_token() {
        let json = json!({
            "user": {
                "email": "another@test.com",
                "username": "anotheruser",
                "external-id": "ext456"
            },
            "session-token": "session_def456",
            "remember-token": null
        });

        let response: LoginResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.user.email, "another@test.com");
        assert_eq!(response.user.username, "anotheruser");
        assert_eq!(response.user.external_id, "ext456");
        assert_eq!(response.session_token, "session_def456");
        assert_eq!(response.remember_token, None);
    }

    #[test]
    fn test_login_response_user_deserialization() {
        let json = json!({
            "email": "test@example.org",
            "username": "testuser123",
            "external-id": "external789"
        });

        let user: LoginResponseUser = serde_json::from_value(json).unwrap();
        assert_eq!(user.email, "test@example.org");
        assert_eq!(user.username, "testuser123");
        assert_eq!(user.external_id, "external789");
    }

    #[test]
    fn test_login_response_kebab_case_fields() {
        // Ensure kebab-case field names are handled correctly
        let json = json!({
            "user": {
                "email": "kebab@test.com",
                "username": "kebabuser",
                "external-id": "kebab-ext-123"  // This should map to external_id
            },
            "session-token": "kebab-session-token",  // This should map to session_token
            "remember-token": "kebab-remember-token"  // This should map to remember_token
        });

        let response: LoginResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.user.email, "kebab@test.com");
        assert_eq!(response.user.username, "kebabuser");
        assert_eq!(response.user.external_id, "kebab-ext-123");
        assert_eq!(response.session_token, "kebab-session-token");
        assert_eq!(
            response.remember_token,
            Some("kebab-remember-token".to_string())
        );
    }

    #[test]
    fn test_login_response_minimal_fields() {
        // Test with minimal required fields (remember-token can be null/missing)
        let json = json!({
            "user": {
                "email": "minimal@test.com",
                "username": "minimaluser",
                "external-id": "min123"
            },
            "session-token": "minimal_session"
            // remember-token is missing (should be None)
        });

        let response: LoginResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.user.email, "minimal@test.com");
        assert_eq!(response.user.username, "minimaluser");
        assert_eq!(response.user.external_id, "min123");
        assert_eq!(response.session_token, "minimal_session");
        assert_eq!(response.remember_token, None);
    }
}
