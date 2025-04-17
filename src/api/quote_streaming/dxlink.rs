use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, broadcast};
use dxlink_rs::feed::events::FeedEvent as DxLinkFeedEvent;
use dxlink_rs::core::auth::DxLinkAuthState;
use dxlink_rs::core::client::DxLinkConnectionState;
use dxlink_rs::websocket_client::{DxLinkWebSocketClient, DxLinkWebSocketClientConfig};
use dxlink_rs::feed::{Feed, FeedContract};
use log::{info, warn, error, debug};

use crate::Result;
use crate::TastyTrade;
use super::error::QuoteStreamingError;
use super::types::{QuoteData, StreamerEvent};

/// A quote streamer implementation using DxLink
/// 
/// This struct manages the WebSocket connection to the quote streaming service
/// and provides methods for subscribing to quotes and receiving events.
pub struct DxLinkQuoteStreamer {
    /// The DxLink feed service instance
    feed: Feed,
    /// The WebSocket client wrapped in an Arc<Mutex> for thread-safe access
    client: Arc<Mutex<DxLinkWebSocketClient>>,
    /// Optional receiver for data events
    receiver: Option<broadcast::Receiver<DxLinkFeedEvent>>,
}

impl DxLinkQuoteStreamer {
    /// Initializes the event receiver
    /// 
    /// This must be called before attempting to receive events.
    pub fn initialize_receiver(&mut self) {
        self.receiver = Some(self.feed.subscribe_to_data_events());
    }

    /// Subscribes to quotes for the given symbols
    /// 
    /// # Arguments
    /// * `symbols` - A slice of symbols to subscribe to
    /// 
    /// # Returns
    /// * `Result<()>` - Ok if subscription was successful, Err otherwise
    pub async fn subscribe_quotes(&self, symbols: &[impl AsRef<str>]) -> Result<()> {
        let subscriptions: Vec<serde_json::Value> = symbols
            .iter()
            .map(|s| serde_json::json!({
                "type": "Quote",
                "symbol": s.as_ref().to_string(),
            }))
            .collect();

        if subscriptions.is_empty() {
            return Ok(());
        }

        self.feed.add_subscriptions(subscriptions).await
            .map_err(|e| QuoteStreamingError::Subscription(format!("dxLink subscription error: {}", e)).into())
    }

    /// Receives the next event from the stream
    /// 
    /// # Returns
    /// * `Result<Option<StreamerEvent>>` - Ok(Some(event)) if an event was received,
    ///   Ok(None) if the event was ignored, Err if an error occurred
    pub async fn receive_event(&mut self) -> Result<Option<StreamerEvent>> {
        let receiver = self.receiver.as_mut()
            .ok_or_else(|| QuoteStreamingError::Streamer(
                "Streamer receiver not initialized. Call initialize_receiver first.".to_string()
            ))?;

        loop {
            match receiver.recv().await {
                Ok(feed_event) => {
                    match feed_event {
                        DxLinkFeedEvent::Quote(quote_event) => {
                            let quote_data = QuoteData {
                                symbol: quote_event.event_symbol,
                                bid_price: quote_event.bid_price.map(|jd| jd.to_f64()),
                                ask_price: quote_event.ask_price.map(|jd| jd.to_f64()),
                                bid_size: quote_event.bid_size.map(|jd| jd.to_f64()),
                                ask_size: quote_event.ask_size.map(|jd| jd.to_f64()),
                                event_time: quote_event.event_time,
                            };
                            return Ok(Some(StreamerEvent {
                                event_type: "Quote".to_string(),
                                data: quote_data,
                            }));
                        }
                        _ => {
                            debug!("Ignoring non-Quote dxLink event: {:?}", feed_event);
                            continue;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Quote streamer lagged, skipped {} messages.", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(QuoteStreamingError::Event(
                        "Streamer channel unexpectedly closed".to_string()
                    ).into());
                }
            }
        }
    }
}

impl TastyTrade {
    /// Creates a new DxLink quote streamer
    /// 
    /// This method:
    /// 1. Fetches the necessary tokens
    /// 2. Creates and configures the WebSocket client
    /// 3. Establishes the connection
    /// 4. Waits for authentication
    /// 5. Creates the feed service
    /// 
    /// # Returns
    /// * `Result<DxLinkQuoteStreamer>` - The configured quote streamer if successful
    pub async fn create_dxlink_quote_streamer(&self) -> Result<DxLinkQuoteStreamer> {
        // Fetch tokens
        let tokens = self.get("/quote-streamer-tokens").await?;
        
        // Create client configuration
        let config = DxLinkWebSocketClientConfig::default();
        
        // Create and wrap client
        let client = DxLinkWebSocketClient::new(config);
        let client_arc_mutex = Arc::<Mutex<DxLinkWebSocketClient>>::new(Mutex::new(client));
        
        // Add state listeners for debugging
        {
            let mut client_guard = client_arc_mutex.lock().await;
            let _ = client_guard.add_connection_state_listener(Box::new(|new_state, old_state| {
                info!("Connection State Change: {:?} -> {:?}", old_state, new_state);
            })).await;
            let _ = client_guard.add_auth_state_listener(Box::new(|new_state, old_state| {
                info!("Auth State Change: {:?} -> {:?}", old_state, new_state);
            })).await;
            let _ = client_guard.add_error_listener(Box::new(|error| {
                error!("dxLink Error: {:?}", error);
            })).await;
        }
        
        // Set auth token and connect
        client_arc_mutex.lock().await.set_auth_token(tokens.token).await
            .map_err(|e| QuoteStreamingError::Authentication(format!("Failed to set auth token: {}", e)).into())?;
        client_arc_mutex.lock().await.connect(tokens.websocket_url).await
            .map_err(|e| QuoteStreamingError::Connection(format!("Failed to connect: {}", e)).into())?;
        
        // Wait for connection and authorization
        // Wait for connection
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if client_arc_mutex.lock().await.get_connection_state().await == DxLinkConnectionState::Connected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }).await.map_err(|_| QuoteStreamingError::Connection(
            "Timeout waiting for dxLink connection".to_string()
        ).into())?;
        
        // Check and wait for authorization
        if client_arc_mutex.lock().await.get_auth_state().await != DxLinkAuthState::Authorized {
            client_arc_mutex.lock().await.send_auth_message(tokens.token).await
                .map_err(|e| QuoteStreamingError::Authentication(format!("Failed to send auth message: {}", e)).into())?;
            
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    if client_arc_mutex.lock().await.get_auth_state().await == DxLinkAuthState::Authorized {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }).await.map_err(|_| QuoteStreamingError::Authentication(
                "Timeout waiting for dxLink authorization".to_string()
            ).into())?;
        } else {
            warn!("dxLink connection established, but no auth token was set/sent. Assuming auth not required.");
        }
        
        info!("DxLink client connected and authorized.");
        
        // Create feed service
        let feed_service = Feed::new(client_arc_mutex.clone(), FeedContract::Auto, None).await
            .map_err(|e| QuoteStreamingError::Streamer(format!("Failed to create feed service: {}", e)).into())?;
        
        // Return new streamer
        Ok(DxLinkQuoteStreamer {
            feed: feed_service,
            client: client_arc_mutex,
            receiver: None,
        })
    }
} 