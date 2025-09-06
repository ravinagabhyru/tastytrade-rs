//! Quote streaming functionality for tastytrade-rs
//!
//! This module provides real-time quote streaming capabilities using the DxLink protocol.
//! It includes:
//! - Quote data structures and event types
//! - DxLink-based quote streamer implementation
//! - Error handling specific to quote streaming
//!
//! # Example
//! ```rust,no_run
//! use tastytrade_rs::TastyTrade;
//! use dxlink_rs::feed::FeedContract;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let tasty = TastyTrade::login("username", "password", false).await?;
//!     let mut streamer = tasty.create_dxlink_quote_streamer().await?;
//!     
//!     // Create a channel for quotes
//!     let channel_id = streamer.create_channel(FeedContract::Auto, None).await?;
//!
//!     // Subscribe to quotes on the channel
//!     streamer.subscribe_quotes(channel_id, &["AAPL", "SPY"]).await?;
//!
//!     // Receive events
//!     while let Ok(Some((ch_id, event))) = streamer.receive_event().await {
//!         println!("Received quote on channel {}: {:?}", ch_id, event);
//!     }
//!     
//!     Ok(())
//! }
//! ```

mod dxlink;  // Now that dxlink dependency is available
mod error;
mod types;

pub use dxlink::DxLinkQuoteStreamer;
pub use error::QuoteStreamingError;
pub use types::{
    ApiQuoteTokens, ApiQuoteTokensData, DxFeedSymbol, QuoteData, QuoteStreamerTokens, StreamerEvent,
};
