use std::sync::Arc;
use std::time::Duration;
use dxlink_rs::FeedDataFormat;
use tokio::sync::{Mutex, broadcast};
use dxlink_rs::feed::events::FeedEvent as DxLinkFeedEvent;
use dxlink_rs::core::auth::DxLinkAuthState;
use dxlink_rs::core::client::DxLinkConnectionState;
use dxlink_rs::websocket_client::{DxLinkWebSocketClient, DxLinkWebSocketClientConfig};
use dxlink_rs::feed::{Feed, FeedContract};
use log::{info, warn, error, debug};

use crate::Result;
use crate::TastyTrade;
use crate::api::base::TastyError;
use super::error::QuoteStreamingError;
use super::types::{ApiQuoteTokens, QuoteData, StreamerEvent};

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
        debug!("Initializing DxLink event receiver");
        self.receiver = Some(self.feed.subscribe_to_data_events());
        debug!("DxLink event receiver initialized successfully");
    }

    /// Subscribes to quotes for the given symbols
    ///
    /// # Arguments
    /// * `symbols` - A slice of symbols to subscribe to
    ///
    /// # Returns
    /// * `Result<()>` - Ok if subscription was successful, Err otherwise
    pub async fn subscribe_quotes(&self, symbols: &[impl AsRef<str>]) -> Result<()> {
        debug!("Subscribing to quotes for symbols: {:?}", symbols.iter().map(|s| s.as_ref()).collect::<Vec<_>>());

        let subscriptions: Vec<serde_json::Value> = symbols
            .iter()
            .map(|s| serde_json::json!({
                "type": "Quote",
                "symbol": s.as_ref().to_string(),
            }))
            .collect();

        if subscriptions.is_empty() {
            debug!("No symbols provided for subscription, returning early");
            return Ok(());
        }

        debug!("Sending subscription request for {} symbols", subscriptions.len());
        self.feed.add_subscriptions(subscriptions).await
            .map_err(|e| -> TastyError {
                error!("Failed to subscribe to quotes: {}", e);
                QuoteStreamingError::Subscription(format!("dxLink subscription error: {}", e)).into()
            })
    }

    /// Receives the next event from the stream
    ///
    /// # Returns
    /// * `Result<Option<StreamerEvent>>` - Ok(Some(event)) if an event was received,
    ///   Ok(None) if the event was ignored, Err if an error occurred
    pub async fn receive_event(&mut self) -> Result<Option<StreamerEvent>> {
        let receiver = self.receiver.as_mut()
            .ok_or_else(|| -> TastyError {
                error!("Attempted to receive events before initializing receiver");
                QuoteStreamingError::Streamer(
                    "Streamer receiver not initialized. Call initialize_receiver first.".to_string()
                ).into()
            })?;

        loop {
            match receiver.recv().await {
                Ok(feed_event) => {
                    match feed_event {
                        DxLinkFeedEvent::Quote(quote_event) => {
                            debug!("Received quote event for symbol: {}", quote_event.event_symbol);
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
                    error!("Streamer channel unexpectedly closed");
                    return Err(TastyError::from(QuoteStreamingError::Event(
                        "Streamer channel unexpectedly closed".to_string()
                    )));
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
        info!("Starting to create DxLink quote streamer");

        // Fetch tokens
        info!("Fetching API quote tokens");
        let tokens = match self.get::<ApiQuoteTokens, _>("/api-quote-tokens").await {
            Ok(t) => {
                info!("Successfully fetched API quote tokens");
                debug!("Using dxLink URL: {}", t.data.dxlink_url);
                debug!("Token length: {}", t.data.token.len());
                t
            },
            Err(e) => {
                error!("Failed to fetch API quote tokens: {}", e);
                error!("Error details: {:?}", e);
                return Err(e);
            }
        };

        // Create client configuration
        debug!("Creating DxLink WebSocket client configuration");
        let config = DxLinkWebSocketClientConfig::default();

        // Create and wrap client
        debug!("Creating DxLink WebSocket client");
        let client = DxLinkWebSocketClient::new(config);
        let client_arc_mutex = Arc::<Mutex<DxLinkWebSocketClient>>::new(Mutex::new(client));

        // Add state listeners for debugging
        info!("Setting up DxLink client state listeners");
        {
            let client_guard = client_arc_mutex.lock().await;
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
        info!("Setting auth token");
        match client_arc_mutex.lock().await.set_auth_token(tokens.data.token.clone()).await {
            Ok(_) => debug!("Auth token set successfully"),
            Err(e) => {
                error!("Failed to set auth token: {}", e);
                return Err(TastyError::from(QuoteStreamingError::Authentication(format!("Failed to set auth token: {}", e))));
            }
        }

        info!("Connecting to WebSocket at URL: {}", tokens.data.dxlink_url);
        match client_arc_mutex.lock().await.connect(tokens.data.dxlink_url.clone()).await {
            Ok(_) => debug!("WebSocket connect request sent successfully"),
            Err(e) => {
                error!("Failed to connect to WebSocket: {}", e);
                return Err(TastyError::from(QuoteStreamingError::Connection(format!("Failed to connect: {}", e))));
            }
        }

        // Wait for connection
        info!("Waiting for WebSocket connection to establish");
        match tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let state = client_arc_mutex.lock().await.get_connection_state().await;
                debug!("Current connection state: {:?}", state);
                if state == DxLinkConnectionState::Connected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }).await {
            Ok(_) => info!("WebSocket connection established"),
            Err(_) => {
                error!("Timeout waiting for dxLink connection");
                return Err(TastyError::from(QuoteStreamingError::Connection(
                    "Timeout waiting for dxLink connection".to_string()
                )));
            }
        }

        // Check and wait for authorization
        info!("Checking authentication state");
        let auth_state = client_arc_mutex.lock().await.get_auth_state().await;
        debug!("Current auth state: {:?}", auth_state);

        if auth_state != DxLinkAuthState::Authorized {
            info!("Not authorized yet, sending auth message");
            match client_arc_mutex.lock().await.send_auth_message(tokens.data.token.clone()).await {
                Ok(_) => debug!("Auth message sent successfully"),
                Err(e) => {
                    error!("Failed to send auth message: {}", e);
                    return Err(TastyError::from(QuoteStreamingError::Authentication(format!("Failed to send auth message: {}", e))));
                }
            }

            info!("Waiting for authorization");
            match tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    let state = client_arc_mutex.lock().await.get_auth_state().await;
                    debug!("Current auth state: {:?}", state);
                    if state == DxLinkAuthState::Authorized {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }).await {
                Ok(_) => info!("DxLink client authorized successfully"),
                Err(_) => {
                    error!("Timeout waiting for dxLink authorization");
                    return Err(TastyError::from(QuoteStreamingError::Authentication(
                        "Timeout waiting for dxLink authorization".to_string()
                    )));
                }
            }
        } else {
            warn!("dxLink connection established, but no auth token was set/sent. Assuming auth not required.");
        }

        info!("DxLink client connected and authorized.");

        // Create feed service
        info!("Creating feed service");
        debug!("Feed configuration: Contract=Auto, Format=Full");
        let feed_service = match Feed::new(client_arc_mutex.clone(), FeedContract::Auto, None, Some(FeedDataFormat::Full)).await {
            Ok(feed) => {
                info!("Feed service created successfully");
                feed
            },
            Err(e) => {
                error!("Failed to create feed service: {}", e);
                error!("Feed service error details: {:?}", e);
                return Err(TastyError::from(QuoteStreamingError::Streamer(format!("Failed to create feed service: {}", e))));
            }
        };

        // Return new streamer
        info!("DxLink quote streamer created successfully");
        Ok(DxLinkQuoteStreamer {
            feed: feed_service,
            client: client_arc_mutex,
            receiver: None,
        })
    }
}
