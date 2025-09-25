use dxlink_rs::DxLinkError;
#[cfg(test)]
use dxlink_rs::DxLinkErrorType;
use thiserror::Error;

/// Errors that can occur during quote streaming operations
#[derive(Debug, Error)]
pub enum QuoteStreamingError {
    /// Errors originating from the DxLink library
    #[error("DxLink error: {0}")]
    DxLink(#[from] DxLinkError),

    /// Errors related to streamer initialization and operation
    #[error("Streamer error: {0}")]
    Streamer(String),

    /// Errors related to WebSocket connection
    #[error("Connection error: {0}")]
    Connection(String),

    /// Errors related to authentication
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Errors related to subscription management
    #[error("Subscription error: {0}")]
    Subscription(String),

    /// Errors related to event handling
    #[error("Event handling error: {0}")]
    Event(String),
}

/// Convert from QuoteStreamingError to TastyError
impl From<QuoteStreamingError> for crate::api::base::TastyError {
    fn from(error: QuoteStreamingError) -> Self {
        use QuoteStreamingError::*;
        let (code, message) = match error {
            DxLink(e) => ("DXLINK_ERROR", format!("DxLink error: {}", e)),
            Streamer(msg) => ("STREAMER_ERROR", msg),
            Connection(msg) => ("CONNECTION_ERROR", msg),
            Authentication(msg) => ("AUTH_ERROR", msg),
            Subscription(msg) => ("SUBSCRIPTION_ERROR", msg),
            Event(msg) => ("EVENT_ERROR", msg),
        };
        crate::api::base::TastyError::Api(crate::api::base::ApiError {
            code: Some(code.to_string()),
            message,
            errors: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::base::TastyError;

    #[test]
    fn test_dxlink_error_conversion() {
        let dxlink_error = DxLinkError::new(DxLinkErrorType::Unknown, "Connection failed");
        let quote_error = QuoteStreamingError::DxLink(dxlink_error);
        let tasty_error: TastyError = quote_error.into();

        match tasty_error {
            TastyError::Api(api_error) => {
                assert_eq!(api_error.code, Some("DXLINK_ERROR".to_string()));
                assert!(api_error.message.contains("DxLink error"));
                assert!(api_error.message.contains("Connection failed"));
                assert!(api_error.errors.is_none());
            }
            _ => panic!("Expected TastyError::Api"),
        }
    }

    #[test]
    fn test_streamer_error_conversion() {
        let quote_error =
            QuoteStreamingError::Streamer("Streamer initialization failed".to_string());
        let tasty_error: TastyError = quote_error.into();

        match tasty_error {
            TastyError::Api(api_error) => {
                assert_eq!(api_error.code, Some("STREAMER_ERROR".to_string()));
                assert_eq!(api_error.message, "Streamer initialization failed");
                assert!(api_error.errors.is_none());
            }
            _ => panic!("Expected TastyError::Api"),
        }
    }

    #[test]
    fn test_connection_error_conversion() {
        let quote_error = QuoteStreamingError::Connection("WebSocket connection lost".to_string());
        let tasty_error: TastyError = quote_error.into();

        match tasty_error {
            TastyError::Api(api_error) => {
                assert_eq!(api_error.code, Some("CONNECTION_ERROR".to_string()));
                assert_eq!(api_error.message, "WebSocket connection lost");
                assert!(api_error.errors.is_none());
            }
            _ => panic!("Expected TastyError::Api"),
        }
    }

    #[test]
    fn test_authentication_error_conversion() {
        let quote_error = QuoteStreamingError::Authentication("Invalid credentials".to_string());
        let tasty_error: TastyError = quote_error.into();

        match tasty_error {
            TastyError::Api(api_error) => {
                assert_eq!(api_error.code, Some("AUTH_ERROR".to_string()));
                assert_eq!(api_error.message, "Invalid credentials");
                assert!(api_error.errors.is_none());
            }
            _ => panic!("Expected TastyError::Api"),
        }
    }

    #[test]
    fn test_subscription_error_conversion() {
        let quote_error =
            QuoteStreamingError::Subscription("Subscription limit exceeded".to_string());
        let tasty_error: TastyError = quote_error.into();

        match tasty_error {
            TastyError::Api(api_error) => {
                assert_eq!(api_error.code, Some("SUBSCRIPTION_ERROR".to_string()));
                assert_eq!(api_error.message, "Subscription limit exceeded");
                assert!(api_error.errors.is_none());
            }
            _ => panic!("Expected TastyError::Api"),
        }
    }

    #[test]
    fn test_event_error_conversion() {
        let quote_error = QuoteStreamingError::Event("Event processing failed".to_string());
        let tasty_error: TastyError = quote_error.into();

        match tasty_error {
            TastyError::Api(api_error) => {
                assert_eq!(api_error.code, Some("EVENT_ERROR".to_string()));
                assert_eq!(api_error.message, "Event processing failed");
                assert!(api_error.errors.is_none());
            }
            _ => panic!("Expected TastyError::Api"),
        }
    }
}
