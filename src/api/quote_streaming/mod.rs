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
//!
//! #[tokio::main]
//! async fn main() {
//!     let tasty = TastyTrade::login("username", "password", false).await?;
//!     let mut streamer = tasty.create_dxlink_quote_streamer().await?;
//!     streamer.initialize_receiver();
//!
//!     streamer.subscribe_quotes(&["AAPL", "SPY"]).await?;
//!
//!     while let Some(event) = streamer.receive_event().await? {
//!         println!("Received quote: {:?}", event);
//!     }
//! }
//! ```

mod error;
mod types;
mod dxlink;

pub use error::QuoteStreamingError;
pub use types::{QuoteData, StreamerEvent, QuoteStreamerTokens, ApiQuoteTokens, ApiQuoteTokensData, DxFeedSymbol};
pub use dxlink::DxLinkQuoteStreamer;
