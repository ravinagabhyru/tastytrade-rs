use serde::{Deserialize, Serialize};

/// Response from the /api-quote-tokens endpoint
#[derive(Debug, Deserialize)]
pub struct ApiQuoteTokens {
    pub data: ApiQuoteTokensData,
    pub context: String,
}

/// Data nested within the ApiQuoteTokens response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ApiQuoteTokensData {
    /// Authentication token for the quote streamer
    pub token: String,
    /// URL for the quote streamer service, renamed from "dxlink-url" in the API
    pub dxlink_url: String,
    /// Service level/plan of the quote streamer
    pub level: String,
}

/// Tokens required for establishing a quote streaming connection
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct QuoteStreamerTokens {
    /// Authentication token for the quote streamer
    pub token: String,
    /// URL for the quote streamer service
    pub streamer_url: String,
    /// WebSocket URL for establishing the connection
    pub websocket_url: String,
    /// Service level/plan of the quote streamer
    pub level: String,
}

/// Represents a single quote data point
#[derive(Debug, Clone)]
pub struct QuoteData {
    /// The symbol being quoted
    pub symbol: String,
    /// Current bid price
    pub bid_price: Option<f64>,
    /// Current ask price
    pub ask_price: Option<f64>,
    /// Size of the current bid
    pub bid_size: Option<f64>,
    /// Size of the current ask
    pub ask_size: Option<f64>,
    /// Timestamp of the quote event
    pub event_time: Option<u64>,
}

/// Represents Greeks data for an option
#[derive(Debug, Clone)]
pub struct GreeksData {
    /// The symbol being quoted
    pub symbol: String,
    /// Implied volatility
    pub volatility: Option<f64>,
    /// Delta (price sensitivity to underlying price change)
    pub delta: Option<f64>,
    /// Gamma (delta sensitivity to underlying price change)
    pub gamma: Option<f64>,
    /// Theta (time decay)
    pub theta: Option<f64>,
    /// Rho (interest rate sensitivity)
    pub rho: Option<f64>,
    /// Vega (volatility sensitivity)
    pub vega: Option<f64>,
    /// Timestamp of the Greeks event
    pub event_time: Option<u64>,
}

/// Different types of streaming data
#[derive(Debug, Clone)]
pub enum StreamerEventData {
    Quote(QuoteData),
    Greeks(GreeksData),
}

/// Represents an event from the quote streamer
#[derive(Debug, Clone)]
pub struct StreamerEvent {
    /// Type of the event (e.g., "Quote", "Greeks")
    pub event_type: String,
    /// The actual data associated with the event
    pub data: StreamerEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxFeedSymbol(pub String);
