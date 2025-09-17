use dxlink_rs::core::auth::DxLinkAuthState;
use dxlink_rs::core::client::DxLinkConnectionState;
use dxlink_rs::feed::events::FeedEvent as DxLinkFeedEvent;
use dxlink_rs::feed::{Feed, FeedContract};
use dxlink_rs::websocket_client::{DxLinkWebSocketClient, DxLinkWebSocketClientConfig};
use dxlink_rs::FeedDataFormat;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, error, info, warn};

use super::error::QuoteStreamingError;
use super::types::{GreeksData, QuoteData, StreamerEvent, StreamerEventData};
use crate::api::base::TastyError;
use crate::api::quote_streaming::ApiQuoteTokensData;
use crate::Result;
use crate::TastyTrade;

/// A quote streamer implementation using DxLink
///
/// This struct manages the underlying WebSocket connection to the quote streaming service
/// using the `dxlink-rs` library. It provides methods for creating logical channels
/// (each represented by a `dxlink_rs::feed::Feed` instance), subscribing to data feeds
/// (like Quotes and Trades) on those channels, and receiving events.
pub struct DxLinkQuoteStreamer {
    /// Map of channel ID to Feed service instance
    feeds: HashMap<u64, Feed>,
    /// The WebSocket client wrapped in an Arc<Mutex> for thread-safe access
    client: Arc<Mutex<DxLinkWebSocketClient>>,
    /// Map of channel ID to receiver for data events
    receivers: HashMap<u64, broadcast::Receiver<DxLinkFeedEvent>>,
}

impl DxLinkQuoteStreamer {
    /// Creates a new feed channel with the specified contract type
    ///
    /// # Arguments
    /// * `contract` - The contract type for the feed
    /// * `data_format` - Optional data format (defaults to Full if None)
    ///
    /// # Returns
    /// * `Result<u64>` - The channel ID on success, error otherwise
    pub async fn create_channel(
        &mut self,
        contract: FeedContract,
        data_format: Option<FeedDataFormat>,
    ) -> Result<u64> {
        debug!("Creating new feed channel");

        // Create feed options for subscription batching
        let options = Some(dxlink_rs::feed::FeedOptions {
            batch_subscriptions_time: 100, // milliseconds
            max_send_subscription_chunk_size: 100,
        });

        match Feed::new(self.client.clone(), contract, options, data_format).await {
            Ok(feed) => {
                // Generate a unique channel ID
                let channel_id = self.generate_channel_id();
                debug!("Successfully created feed for channel {}", channel_id);

                // Store the feed
                self.feeds.insert(channel_id, feed);

                // Subscribe to feed events
                let receiver = self
                    .feeds
                    .get(&channel_id)
                    .expect("Feed was just inserted")
                    .subscribe_to_data_events();

                self.receivers.insert(channel_id, receiver);

                // Set up the feed with FEED_SETUP message
                // self.setup_feed(channel_id).await?;

                debug!("Channel {} created successfully", channel_id);
                Ok(channel_id)
            }
            Err(e) => {
                error!("Failed to create feed service: {}", e);
                Err(TastyError::from(QuoteStreamingError::Streamer(format!(
                    "Failed to create feed service: {}",
                    e
                ))))
            }
        }
    }

    /// Generates a unique channel ID
    fn generate_channel_id(&self) -> u64 {
        static CHANNEL_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        CHANNEL_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Creates a feed channel with the Auto contract type and Full data format
    ///
    /// # Returns
    /// * `Result<u64>` - The channel ID on success, error otherwise
    pub async fn create_default_channel(&mut self) -> Result<u64> {
        self.create_channel(FeedContract::Auto, Some(FeedDataFormat::Full))
            .await
    }

    /// Closes a logical channel by its ID by removing the associated feed and receiver
    /// from the streamer's internal tracking.
    ///
    /// This stops the streamer from processing further events for this channel ID.
    ///
    /// # Arguments
    /// * `channel_id` - The ID of the channel to close
    ///
    /// # Returns
    /// * `Result<()>` - Ok if successful, Err otherwise
    pub async fn close_channel(&mut self, channel_id: u64) -> Result<()> {
        if !self.feeds.contains_key(&channel_id) {
            return Err(TastyError::from(QuoteStreamingError::Streamer(format!(
                "Channel {} does not exist",
                channel_id
            ))));
        }

        debug!("Closing channel {}", channel_id);

        // There's no close_channel method on the client
        // We'll just remove our references to the feed, channel, and receiver

        // Remove the feed, channel, and receiver for this channel
        self.feeds.remove(&channel_id);
        self.receivers.remove(&channel_id);

        debug!("Channel {} closed successfully", channel_id);
        Ok(())
    }

    /// Subscribes to a specific event type for the given symbols on a channel
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID to subscribe on
    /// * `event_type` - The type of event to subscribe to (e.g., "Quote", "Trade")
    /// * `symbols` - A slice of symbols to subscribe to
    ///
    /// # Returns
    /// * `Result<()>` - Ok if subscription was successful, Err otherwise
    pub async fn subscribe(
        &self,
        channel_id: u64,
        event_type: &str,
        symbols: &[impl AsRef<str>],
    ) -> Result<()> {
        if !self.feeds.contains_key(&channel_id) {
            return Err(TastyError::from(QuoteStreamingError::Streamer(format!(
                "Channel {} does not exist",
                channel_id
            ))));
        }

        debug!(
            "Subscribing to {} events for symbols on channel {}: {:?}",
            event_type,
            channel_id,
            symbols.iter().map(|s| s.as_ref()).collect::<Vec<_>>()
        );

        let subscriptions: Vec<serde_json::Value> = symbols
            .iter()
            .map(|s| {
                serde_json::json!({
                    "type": event_type,
                    "symbol": s.as_ref().to_string(),
                })
            })
            .collect();

        if subscriptions.is_empty() {
            debug!("No symbols provided for subscription, returning early");
            return Ok(());
        }

        debug!(
            "Sending subscription request for {} symbols",
            subscriptions.len()
        );
        debug!(
            "Subscription payload: {}",
            serde_json::to_string_pretty(&subscriptions).unwrap()
        );

        let feed = self
            .feeds
            .get(&channel_id)
            .expect("Channel existence was just checked");

        // Also send through the feed's add_subscriptions method
        feed.add_subscriptions(subscriptions)
            .await
            .map_err(|e| -> TastyError {
                error!("Failed to subscribe to {}: {}", event_type, e);
                QuoteStreamingError::Subscription(format!("dxLink subscription error: {}", e))
                    .into()
            })?;

        debug!(
            "Successfully sent subscription requests for channel {}",
            channel_id
        );
        Ok(())
    }

    /// Subscribes to quotes for the given symbols on a channel
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID to subscribe on
    /// * `symbols` - A slice of symbols to subscribe to
    ///
    /// # Returns
    /// * `Result<()>` - Ok if subscription was successful, Err otherwise
    pub async fn subscribe_quotes(
        &self,
        channel_id: u64,
        symbols: &[impl AsRef<str>],
    ) -> Result<()> {
        self.subscribe(channel_id, "Quote", symbols).await
    }

    /// Subscribes to trades for the given symbols on a channel
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID to subscribe on
    /// * `symbols` - A slice of symbols to subscribe to
    ///
    /// # Returns
    /// * `Result<()>` - Ok if subscription was successful, Err otherwise
    pub async fn subscribe_trades(
        &self,
        channel_id: u64,
        symbols: &[impl AsRef<str>],
    ) -> Result<()> {
        self.subscribe(channel_id, "Trade", symbols).await
    }

    /// Subscribes to Greeks for the given symbols on a channel
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID to subscribe on
    /// * `symbols` - A slice of symbols to subscribe to
    ///
    /// # Returns
    /// * `Result<()>` - Ok if subscription was successful, Err otherwise
    pub async fn subscribe_greeks(
        &self,
        channel_id: u64,
        symbols: &[impl AsRef<str>],
    ) -> Result<()> {
        self.subscribe(channel_id, "Greeks", symbols).await
    }

    /// Unsubscribes from a specific event type for the given symbols on a channel
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID to unsubscribe from
    /// * `event_type` - The type of event to unsubscribe from
    /// * `symbols` - A slice of symbols to unsubscribe from
    ///
    /// # Returns
    /// * `Result<()>` - Ok if unsubscription was successful, Err otherwise
    pub async fn unsubscribe(
        &self,
        channel_id: u64,
        event_type: &str,
        symbols: &[impl AsRef<str>],
    ) -> Result<()> {
        if !self.feeds.contains_key(&channel_id) {
            return Err(TastyError::from(QuoteStreamingError::Streamer(format!(
                "Channel {} does not exist",
                channel_id
            ))));
        }

        debug!(
            "Unsubscribing from {} events for symbols on channel {}: {:?}",
            event_type,
            channel_id,
            symbols.iter().map(|s| s.as_ref()).collect::<Vec<_>>()
        );

        let subscriptions: Vec<serde_json::Value> = symbols
            .iter()
            .map(|s| {
                serde_json::json!({
                    "type": event_type,
                    "symbol": s.as_ref().to_string(),
                })
            })
            .collect();

        if subscriptions.is_empty() {
            debug!("No symbols provided for unsubscription, returning early");
            return Ok(());
        }

        debug!(
            "Sending unsubscription request for {} symbols",
            subscriptions.len()
        );
        let feed = self
            .feeds
            .get(&channel_id)
            .expect("Channel existence was just checked");

        feed.remove_subscriptions(subscriptions)
            .await
            .map_err(|e| -> TastyError {
                error!("Failed to unsubscribe from {}: {}", event_type, e);
                QuoteStreamingError::Subscription(format!("dxLink unsubscription error: {}", e))
                    .into()
            })
    }

    /// Resets all subscriptions on a specific channel by removing all current subscriptions
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID to reset subscriptions for
    ///
    /// # Returns
    /// * `Result<()>` - Ok if reset was successful, Err otherwise
    pub async fn reset_subscriptions(&self, channel_id: u64) -> Result<()> {
        if !self.feeds.contains_key(&channel_id) {
            return Err(TastyError::from(QuoteStreamingError::Streamer(format!(
                "Channel {} does not exist",
                channel_id
            ))));
        }

        debug!("Resetting all subscriptions on channel {}", channel_id);
        let feed = self
            .feeds
            .get(&channel_id)
            .expect("Channel existence was just checked");

        // Since there is no direct reset_subscriptions method, we'll use remove_subscriptions
        // with an empty vector to request removal of all subscriptions
        feed.remove_subscriptions(vec![])
            .await
            .map_err(|e| -> TastyError {
                error!("Failed to reset subscriptions: {}", e);
                QuoteStreamingError::Subscription(format!("dxLink subscription reset error: {}", e))
                    .into()
            })
    }

    /// Receives the next event from a specific channel
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID to receive from
    ///
    /// # Returns
    /// * `Result<Option<StreamerEvent>>` - Ok(Some(event)) if an event was received,
    ///   Ok(None) if the event was ignored, Err if an error occurred
    pub async fn receive_event_from_channel(
        &mut self,
        channel_id: u64,
    ) -> Result<Option<StreamerEvent>> {
        let receiver = self
            .receivers
            .get_mut(&channel_id)
            .ok_or_else(|| -> TastyError {
                error!(
                    "Attempted to receive events from non-existent channel {}",
                    channel_id
                );
                QuoteStreamingError::Streamer(format!("Channel {} does not exist", channel_id))
                    .into()
            })?;

        loop {
            match receiver.recv().await {
                Ok(feed_event) => {
                    // Process the event based on its type
                    match &feed_event {
                        DxLinkFeedEvent::Quote(quote_event) => {
                            debug!(
                                "Received quote event for symbol: {} on channel {}",
                                quote_event.event_symbol, channel_id
                            );
                            let quote_data = QuoteData {
                                symbol: quote_event.event_symbol.clone(),
                                bid_price: quote_event.bid_price.as_ref().map(|jd| jd.to_f64()),
                                ask_price: quote_event.ask_price.as_ref().map(|jd| jd.to_f64()),
                                bid_size: quote_event.bid_size.as_ref().map(|jd| jd.to_f64()),
                                ask_size: quote_event.ask_size.as_ref().map(|jd| jd.to_f64()),
                                event_time: quote_event.event_time,
                            };
                            return Ok(Some(StreamerEvent {
                                event_type: "Quote".to_string(),
                                data: StreamerEventData::Quote(quote_data),
                            }));
                        }
                        DxLinkFeedEvent::Trade(trade_event) => {
                            debug!(
                                "Received trade event for symbol: {} on channel {}",
                                trade_event.event_symbol, channel_id
                            );
                            // For now, we convert trade events to the same QuoteData structure
                            // In a real implementation, you might want to create a more specific type
                            let quote_data = QuoteData {
                                symbol: trade_event.event_symbol.clone(),
                                bid_price: None,
                                ask_price: trade_event.price.as_ref().map(|jd| jd.to_f64()),
                                bid_size: None,
                                ask_size: trade_event.size.as_ref().map(|jd| jd.to_f64()),
                                event_time: trade_event.event_time,
                            };
                            return Ok(Some(StreamerEvent {
                                event_type: "Trade".to_string(),
                                data: StreamerEventData::Quote(quote_data),
                            }));
                        }
                        DxLinkFeedEvent::Greeks(greeks_event) => {
                            debug!(
                                "Received Greeks event for symbol: {} on channel {}",
                                greeks_event.event_symbol, channel_id
                            );
                            let greeks_data = GreeksData {
                                symbol: greeks_event.event_symbol.clone(),
                                volatility: greeks_event.volatility.as_ref().map(|jd| jd.to_f64()),
                                delta: greeks_event.delta.as_ref().map(|jd| jd.to_f64()),
                                gamma: greeks_event.gamma.as_ref().map(|jd| jd.to_f64()),
                                theta: greeks_event.theta.as_ref().map(|jd| jd.to_f64()),
                                rho: greeks_event.rho.as_ref().map(|jd| jd.to_f64()),
                                vega: greeks_event.vega.as_ref().map(|jd| jd.to_f64()),
                                event_time: greeks_event.event_time,
                            };
                            return Ok(Some(StreamerEvent {
                                event_type: "Greeks".to_string(),
                                data: StreamerEventData::Greeks(greeks_data),
                            }));
                        }
                        _ => {
                            debug!(
                                "Ignoring unsupported dxLink event type on channel {}: {:?}",
                                channel_id, feed_event
                            );
                            continue;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(
                        "Quote streamer on channel {} lagged, skipped {} messages.",
                        channel_id, n
                    );
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    error!("Streamer channel {} unexpectedly closed", channel_id);
                    return Err(TastyError::from(QuoteStreamingError::Event(format!(
                        "Streamer channel {} unexpectedly closed",
                        channel_id
                    ))));
                }
            }
        }
    }

    /// Receives the next event from any channel
    ///
    /// # Returns
    /// * `Result<Option<(u64, StreamerEvent)>>` - Ok(Some((channel_id, event))) if an event was received,
    ///   Ok(None) if all channels are closed, Err if an error occurred
    pub async fn receive_event(&mut self) -> Result<Option<(u64, StreamerEvent)>> {
        if self.receivers.is_empty() {
            return Err(TastyError::from(QuoteStreamingError::Streamer(
                "No channels available to receive events from".to_string(),
            )));
        }

        // Collect all channel IDs first to avoid borrowing issues
        let channel_ids: Vec<u64> = self.receivers.keys().copied().collect();

        // Iterate over the collected channel IDs
        for channel_id in channel_ids {
            // Check if this channel still exists (might have been removed by another iteration)
            if !self.receivers.contains_key(&channel_id) {
                continue;
            }

            match self.receive_event_from_channel(channel_id).await {
                Ok(Some(event)) => return Ok(Some((channel_id, event))),
                Ok(None) => continue,
                Err(e) => {
                    // Check if this is an API error about closed channels
                    match &e {
                        TastyError::Api(api_err)
                            if api_err.message.contains("unexpectedly closed") =>
                        {
                            // Channel is closed, remove it
                            self.receivers.remove(&channel_id);
                            continue;
                        }
                        _ => return Err(e),
                    }
                }
            }
        }

        // If we got here, we checked all channels and none had events
        Ok(None)
    }

    /// Get all active channel IDs
    ///
    /// # Returns
    /// * `Vec<u64>` - List of active channel IDs
    pub fn get_channel_ids(&self) -> Vec<u64> {
        self.feeds.keys().copied().collect()
    }
}

impl TastyTrade {
    /// Creates a new DxLink quote streamer with multi-channel support
    ///
    /// This method:
    /// 1. Fetches the necessary tokens
    /// 2. Creates and configures the WebSocket client
    /// 3. Establishes the connection
    /// 4. Waits for authentication
    /// 5. Returns the streamer instance (channels can be created separately)
    ///
    /// # Returns
    /// * `Result<DxLinkQuoteStreamer>` - The configured quote streamer if successful
    pub async fn create_dxlink_quote_streamer(&self) -> Result<DxLinkQuoteStreamer> {
        info!("Starting to create DxLink quote streamer");

        // Fetch tokens
        info!("Fetching API quote tokens");
        let tokens = match self.get::<ApiQuoteTokensData, _>("/api-quote-tokens").await {
            Ok(t) => {
                info!("Successfully fetched API quote tokens");
                debug!("Using dxLink URL: {}", t.dxlink_url);
                debug!("Token length: {}", t.token.len());
                t
            }
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
            let _ = client_guard
                .add_connection_state_listener(Box::new(|new_state, old_state| {
                    info!(
                        "Connection State Change: {:?} -> {:?}",
                        old_state, new_state
                    );
                }))
                .await;
            let _ = client_guard
                .add_auth_state_listener(Box::new(|new_state, old_state| {
                    info!("Auth State Change: {:?} -> {:?}", old_state, new_state);
                }))
                .await;
            let _ = client_guard
                .add_error_listener(Box::new(|error| {
                    error!("dxLink Error: {:?}", error);
                }))
                .await;
        }

        // Set auth token and connect
        info!("Setting auth token");
        match client_arc_mutex
            .lock()
            .await
            .set_auth_token(tokens.token.clone())
            .await
        {
            Ok(_) => debug!("Auth token set successfully"),
            Err(e) => {
                error!("Failed to set auth token: {}", e);
                return Err(TastyError::from(QuoteStreamingError::Authentication(
                    format!("Failed to set auth token: {}", e),
                )));
            }
        }

        info!("Connecting to WebSocket at URL: {}", tokens.dxlink_url);
        match client_arc_mutex
            .lock()
            .await
            .connect(tokens.dxlink_url.clone())
            .await
        {
            Ok(_) => debug!("WebSocket connect request sent successfully"),
            Err(e) => {
                error!("Failed to connect to WebSocket: {}", e);
                return Err(TastyError::from(QuoteStreamingError::Connection(format!(
                    "Failed to connect: {}",
                    e
                ))));
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
        })
        .await
        {
            Ok(_) => info!("WebSocket connection established"),
            Err(_) => {
                error!("Timeout waiting for dxLink connection");
                return Err(TastyError::from(QuoteStreamingError::Connection(
                    "Timeout waiting for dxLink connection".to_string(),
                )));
            }
        }

        // Check and wait for authorization
        info!("Checking authentication state");
        let auth_state = client_arc_mutex.lock().await.get_auth_state().await;
        debug!("Current auth state: {:?}", auth_state);

        if auth_state != DxLinkAuthState::Authorized {
            info!("Not authorized yet, sending auth message");
            match client_arc_mutex
                .lock()
                .await
                .send_auth_message(tokens.token.clone())
                .await
            {
                Ok(_) => debug!("Auth message sent successfully"),
                Err(e) => {
                    error!("Failed to send auth message: {}", e);
                    return Err(TastyError::from(QuoteStreamingError::Authentication(
                        format!("Failed to send auth message: {}", e),
                    )));
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
            })
            .await
            {
                Ok(_) => info!("DxLink client authorized successfully"),
                Err(_) => {
                    error!("Timeout waiting for dxLink authorization");
                    return Err(TastyError::from(QuoteStreamingError::Authentication(
                        "Timeout waiting for dxLink authorization".to_string(),
                    )));
                }
            }
        } else {
            warn!("dxLink connection established, but no auth token was set/sent. Assuming auth not required.");
        }

        info!("DxLink client connected and authorized.");

        // Return new streamer with empty channel collections
        info!("Creating DxLink quote streamer instance");
        Ok(DxLinkQuoteStreamer {
            feeds: HashMap::new(),
            client: client_arc_mutex,
            receivers: HashMap::new(),
        })
    }
}
