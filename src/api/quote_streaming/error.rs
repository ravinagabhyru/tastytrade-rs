use dxlink_rs::DxLinkError;
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
