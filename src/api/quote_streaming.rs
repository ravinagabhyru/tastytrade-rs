use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use dxlink_rs::feed::events::FeedEvent as DxLinkFeedEvent;
use dxlink_rs::core::auth::DxLinkAuthState;
use dxlink_rs::core::client::DxLinkConnectionState;
use dxlink_rs::websocket_client::{DxLinkWebSocketClient, DxLinkWebSocketClientConfig};
use dxlink_rs::feed::{Feed, FeedContract};
use log::{info, warn, error, debug};

use crate::Result;
use crate::TastyTrade;
use crate::api::base::{TastyError, ApiError};

use super::order::AsSymbol;
use super::order::InstrumentType;
use super::order::Symbol;

impl TastyTrade {
    pub async fn quote_streamer_tokens(&self) -> Result<QuoteStreamerTokens> {
        self.get("/quote-streamer-tokens").await
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct QuoteStreamerTokens {
    pub token: String,
    pub streamer_url: String,
    pub websocket_url: String,
    pub level: String,
}

impl TastyTrade {
    pub async fn get_streamer_symbol(
        &self,
        instrument_type: &InstrumentType,
        symbol: &Symbol,
    ) -> Result<String> {
        use InstrumentType::*;
        let sym = match instrument_type {
            Equity => self.get_equity_info(symbol).await?.streamer_symbol,
            EquityOption => self.get_option_info(symbol).await?.streamer_symbol,
            _ => unimplemented!(),
        };
        Ok(sym)
    }
}

pub struct DxLinkQuoteStreamer {
    feed: Feed,
    client: Arc<Mutex<DxLinkWebSocketClient>>,
    receiver: Option<broadcast::Receiver<DxLinkFeedEvent>>,
}

impl DxLinkQuoteStreamer {
    pub fn initialize_receiver(&mut self) {
        self.receiver = Some(self.feed.subscribe_to_data_events());
    }

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
            .map_err(|e| TastyError::Api(ApiError {
                code: Some("SUBSCRIPTION_ERROR".to_string()),
                message: format!("dxLink subscription error: {}", e),
                errors: None,
            }))?;

        Ok(())
    }

    pub async fn receive_event(&mut self) -> Result<Option<StreamerEvent>> {
        let receiver = self.receiver.as_mut()
            .ok_or_else(|| TastyError::Api(ApiError {
                code: Some("RECEIVER_ERROR".to_string()),
                message: "Streamer receiver not initialized. Call initialize_receiver first.".to_string(),
                errors: None,
            }))?;

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
                    return Err(TastyError::Api(ApiError {
                        code: Some("STREAMER_CLOSED".to_string()),
                        message: "Streamer channel unexpectedly closed".to_string(),
                        errors: None,
                    }));
                }
            }
        }
    }
}

impl TastyTrade {
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
            .map_err(|e| TastyError::Api(ApiError {
                code: Some("AUTH_ERROR".to_string()),
                message: format!("Failed to set auth token: {}", e),
                errors: None,
            }))?;
        client_arc_mutex.lock().await.connect(tokens.websocket_url).await
            .map_err(|e| TastyError::Api(ApiError {
                code: Some("CONNECTION_ERROR".to_string()),
                message: format!("Failed to connect: {}", e),
                errors: None,
            }))?;
        
        // Wait for connection and authorization
        use std::time::Duration;
        
        // Wait for connection
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if client_arc_mutex.lock().await.get_connection_state().await == DxLinkConnectionState::Connected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }).await.map_err(|_| TastyError::Api(ApiError {
            code: Some("CONNECTION_TIMEOUT".to_string()),
            message: "Timeout waiting for dxLink connection".to_string(),
            errors: None,
        }))?;
        
        // Check and wait for authorization
        if client_arc_mutex.lock().await.get_auth_state().await != DxLinkAuthState::Authorized {
            // Since last_auth_token is private, we'll use the token we already have
            client_arc_mutex.lock().await.send_auth_message(tokens.token).await
                .map_err(|e| TastyError::Api(ApiError {
                    code: Some("AUTH_ERROR".to_string()),
                    message: format!("Failed to send auth message: {}", e),
                    errors: None,
                }))?;
            
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    if client_arc_mutex.lock().await.get_auth_state().await == DxLinkAuthState::Authorized {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }).await.map_err(|_| TastyError::Api(ApiError {
                code: Some("AUTH_TIMEOUT".to_string()),
                message: "Timeout waiting for dxLink authorization".to_string(),
                errors: None,
            }))?;
        } else {
            warn!("dxLink connection established, but no auth token was set/sent. Assuming auth not required.");
        }
        
        info!("DxLink client connected and authorized.");
        
        // Create feed service
        let feed_service = Feed::new(client_arc_mutex.clone(), FeedContract::Auto, None).await
            .map_err(|e| TastyError::Api(ApiError {
                code: Some("FEED_ERROR".to_string()),
                message: format!("Failed to create feed service: {}", e),
                errors: None,
            }))?;
        
        // Return new streamer
        Ok(DxLinkQuoteStreamer {
            feed: feed_service,
            client: client_arc_mutex,
            receiver: None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct QuoteData {
    pub symbol: String,
    pub bid_price: Option<f64>,
    pub ask_price: Option<f64>,
    pub bid_size: Option<f64>,
    pub ask_size: Option<f64>,
    pub event_time: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct StreamerEvent {
    pub event_type: String,
    pub data: QuoteData,
}
