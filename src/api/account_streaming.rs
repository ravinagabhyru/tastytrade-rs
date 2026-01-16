use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::{
    api::accounts::{Account, Balance},
    api::base::Result,
    TastyTrade,
};

use super::{order::LiveOrderRecord, position::BriefPosition};

static WEBSOCKET_DEMO_URL: &str = "wss://streamer.cert.tastyworks.com";
static WEBSOCKET_URL: &str = "wss://streamer.tastyworks.com";

/// Maximum reconnection attempts before giving up
const MAX_RECONNECT_ATTEMPTS: u32 = 10;
/// Initial backoff delay for reconnection (1 second)
const INITIAL_BACKOFF_MS: u64 = 1000;
/// Maximum backoff delay (30 seconds)
const MAX_BACKOFF_MS: u64 = 30000;
/// Heartbeat interval (30 seconds)
const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Mask account number for logging (shows first 3 and last 2 chars)
fn mask_account(account: &str) -> String {
    if account.len() <= 5 {
        "***".to_string()
    } else {
        format!("{}***{}", &account[..3], &account[account.len() - 2..])
    }
}

/// Mask sensitive data in raw JSON strings (account numbers, session IDs, tokens)
fn mask_sensitive_data(data: &str) -> String {
    use regex::Regex;
    // Match account-number field values (alphanumeric, typically 8 chars like "5WY40297")
    let account_re = Regex::new(r#""account-number"\s*:\s*"([A-Z0-9]{6,})""#).unwrap();
    let result = account_re.replace_all(data, |caps: &regex::Captures| {
        let account = &caps[1];
        format!(r#""account-number":"{}""#, mask_account(account))
    });
    // Also mask web-socket-session-id
    let session_re = Regex::new(r#""web-socket-session-id"\s*:\s*"([^"]+)""#).unwrap();
    let result = session_re.replace_all(&result, r#""web-socket-session-id":"***""#);
    result.to_string()
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum SubRequestAction {
    Heartbeat,
    Connect,
    PublicWatchlistsSubscribe,
    QuoteAlertsSubscribe,
    UserMessageSubscribe,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct SubRequest<T> {
    auth_token: String,
    action: SubRequestAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<T>,
    request_id: u64,
    source: &'static str,
}

/// Source identifier for API requests
const SOURCE: &str = "tastytrade-rs/0.6.1";

pub struct HandlerAction {
    action: SubRequestAction,
    value: Option<Box<dyn erased_serde::Serialize + Send + Sync>>,
    request_id: u64,
}

/// Year-to-date gain summary for an underlying symbol
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct UnderlyingYearGainSummary {
    pub account_number: String,
    pub symbol: String,
    pub instrument_type: String,
    pub year: String,
    pub commissions: String,
    pub commissions_effect: String,
    pub fees: String,
    pub fees_effect: String,
    pub realized_lot_gain: String,
    pub realized_lot_gain_effect: String,
    pub yearly_realized_gain: String,
    pub yearly_realized_gain_effect: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum AccountMessage {
    Order(LiveOrderRecord),
    AccountBalance(Box<Balance>),
    CurrentPosition(Box<BriefPosition>),
    UnderlyingYearGainSummary(UnderlyingYearGainSummary),
    OrderChain,
    ExternalTransaction,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct StatusMessage {
    pub status: String,
    pub action: String,
    pub web_socket_session_id: String,
    pub request_id: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ErrorMessage {
    pub status: String,
    pub action: String,
    pub web_socket_session_id: String,
    pub message: String,
}

/// Response to heartbeat requests (uses ws-sequence instead of request-id)
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct HeartbeatResponse {
    pub status: String,
    pub action: String,
    pub web_socket_session_id: String,
    pub ws_sequence: u64,
}

/// Events emitted by the account streamer, including connection lifecycle events
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Account-related message (order, position, balance updates)
    Account(AccountEvent),
    /// Stream has disconnected
    Disconnected { reason: String },
    /// Stream is attempting to reconnect
    Reconnecting { attempt: u32, max_attempts: u32 },
    /// Stream has successfully reconnected
    Reconnected,
    /// Stream has been closed (either by user or after max reconnection attempts)
    Closed { reason: String },
}

//#[allow(clippy::large_enum_variant)]
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum AccountEvent {
    ErrorMessage(ErrorMessage),
    HeartbeatResponse(HeartbeatResponse),
    StatusMessage(StatusMessage),
    AccountMessage(Box<AccountMessage>),
}

/// Configuration for the account streamer
#[derive(Debug, Clone)]
pub struct AccountStreamerConfig {
    /// Whether to automatically reconnect on disconnect
    pub auto_reconnect: bool,
    /// Maximum number of reconnection attempts (0 = unlimited)
    pub max_reconnect_attempts: u32,
    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds
    pub max_backoff_ms: u64,
}

impl Default for AccountStreamerConfig {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            max_reconnect_attempts: MAX_RECONNECT_ATTEMPTS,
            initial_backoff_ms: INITIAL_BACKOFF_MS,
            max_backoff_ms: MAX_BACKOFF_MS,
        }
    }
}

/// Internal state for managing connection and reconnection
#[derive(Debug)]
struct StreamerState {
    /// Auth token for the connection
    token: String,
    /// Whether this is a demo/sandbox connection
    is_demo: bool,
    /// Accounts to re-subscribe to on reconnect
    subscribed_accounts: Vec<String>,
}

#[derive(Debug)]
pub struct AccountStreamer {
    /// Receiver for stream events (accounts data + lifecycle events)
    pub event_receiver: flume::Receiver<StreamEvent>,
    /// Sender for actions to the streamer
    pub action_sender: flume::Sender<HandlerAction>,
    /// Flag to signal shutdown
    shutdown: Arc<AtomicBool>,
    /// Internal state protected by mutex
    state: Arc<Mutex<StreamerState>>,
    /// Counter for generating unique request IDs
    request_id_counter: Arc<AtomicU64>,
}

impl AccountStreamer {
    /// Connect to the TastyTrade account streaming WebSocket with default configuration
    pub async fn connect(tasty: &TastyTrade) -> Result<AccountStreamer> {
        Self::connect_with_config(tasty, AccountStreamerConfig::default()).await
    }

    /// Connect to the TastyTrade account streaming WebSocket with custom configuration
    pub async fn connect_with_config(
        tasty: &TastyTrade,
        config: AccountStreamerConfig,
    ) -> Result<AccountStreamer> {
        // Capture the current auth header value for websocket auth
        let token = {
            // best-effort snapshot; websocket does not auto-refresh this value
            // if the access token expires, the streaming connection may need reestablishing
            tasty.auth_state.read().await.auth_header()
        };

        let is_demo = tasty.demo;
        let (event_sender, event_receiver) = flume::unbounded();
        let (action_sender, action_receiver): (
            flume::Sender<HandlerAction>,
            flume::Receiver<HandlerAction>,
        ) = flume::unbounded();

        let shutdown = Arc::new(AtomicBool::new(false));
        let state = Arc::new(Mutex::new(StreamerState {
            token: token.clone(),
            is_demo,
            subscribed_accounts: Vec::new(),
        }));

        // Spawn the main connection manager task
        let shutdown_clone = shutdown.clone();
        let state_clone = state.clone();
        let action_sender_clone = action_sender.clone();
        tokio::spawn(async move {
            Self::connection_manager(
                state_clone,
                config,
                event_sender,
                action_receiver,
                action_sender_clone,
                shutdown_clone,
            )
            .await;
        });

        Ok(Self {
            event_receiver,
            action_sender,
            shutdown,
            state,
            request_id_counter: Arc::new(AtomicU64::new(1)),
        })
    }

    /// Main connection manager that handles connection lifecycle and reconnection
    async fn connection_manager(
        state: Arc<Mutex<StreamerState>>,
        config: AccountStreamerConfig,
        event_sender: flume::Sender<StreamEvent>,
        action_receiver: flume::Receiver<HandlerAction>,
        action_sender: flume::Sender<HandlerAction>,
        shutdown: Arc<AtomicBool>,
    ) {
        let mut reconnect_attempt = 0u32;

        loop {
            if shutdown.load(Ordering::SeqCst) {
                info!("AccountStreamer shutdown requested");
                let _ = event_sender.send_async(StreamEvent::Closed {
                    reason: "Shutdown requested".to_string(),
                }).await;
                break;
            }

            // Attempt to connect
            let (is_demo, token) = {
                let s = state.lock().await;
                (s.is_demo, s.token.clone())
            };

            match Self::establish_connection(
                is_demo,
                &token,
                event_sender.clone(),
                action_receiver.clone(),
                action_sender.clone(),
                shutdown.clone(),
            )
            .await
            {
                Ok(disconnect_reason) => {
                    if shutdown.load(Ordering::SeqCst) {
                        info!("AccountStreamer closed after shutdown");
                        break;
                    }

                    // Connection was established but then lost
                    warn!("AccountStreamer disconnected: {}", disconnect_reason);
                    let _ = event_sender.send_async(StreamEvent::Disconnected {
                        reason: disconnect_reason,
                    }).await;

                    if !config.auto_reconnect {
                        let _ = event_sender.send_async(StreamEvent::Closed {
                            reason: "Auto-reconnect disabled".to_string(),
                        }).await;
                        break;
                    }

                    reconnect_attempt += 1;
                    if config.max_reconnect_attempts > 0
                        && reconnect_attempt > config.max_reconnect_attempts
                    {
                        error!(
                            "Max reconnection attempts ({}) reached, giving up",
                            config.max_reconnect_attempts
                        );
                        let _ = event_sender.send_async(StreamEvent::Closed {
                            reason: format!(
                                "Max reconnection attempts ({}) reached",
                                config.max_reconnect_attempts
                            ),
                        }).await;
                        break;
                    }

                    // Calculate backoff with exponential increase
                    let backoff_ms = std::cmp::min(
                        config.initial_backoff_ms * 2u64.pow(reconnect_attempt - 1),
                        config.max_backoff_ms,
                    );

                    info!(
                        "Reconnecting in {}ms (attempt {}/{})",
                        backoff_ms,
                        reconnect_attempt,
                        if config.max_reconnect_attempts > 0 {
                            config.max_reconnect_attempts.to_string()
                        } else {
                            "∞".to_string()
                        }
                    );

                    let _ = event_sender.send_async(StreamEvent::Reconnecting {
                        attempt: reconnect_attempt,
                        max_attempts: config.max_reconnect_attempts,
                    }).await;

                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
                Err(e) => {
                    error!("Failed to establish connection: {}", e);

                    if !config.auto_reconnect {
                        let _ = event_sender.send_async(StreamEvent::Closed {
                            reason: format!("Connection failed: {}", e),
                        }).await;
                        break;
                    }

                    reconnect_attempt += 1;
                    if config.max_reconnect_attempts > 0
                        && reconnect_attempt > config.max_reconnect_attempts
                    {
                        let _ = event_sender.send_async(StreamEvent::Closed {
                            reason: format!(
                                "Max reconnection attempts reached after error: {}",
                                e
                            ),
                        }).await;
                        break;
                    }

                    let backoff_ms = std::cmp::min(
                        config.initial_backoff_ms * 2u64.pow(reconnect_attempt - 1),
                        config.max_backoff_ms,
                    );

                    let _ = event_sender.send_async(StreamEvent::Reconnecting {
                        attempt: reconnect_attempt,
                        max_attempts: config.max_reconnect_attempts,
                    }).await;

                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }

    /// Establish a WebSocket connection and run the event loop
    /// Returns the reason for disconnection if it occurs
    async fn establish_connection(
        is_demo: bool,
        token: &str,
        event_sender: flume::Sender<StreamEvent>,
        action_receiver: flume::Receiver<HandlerAction>,
        action_sender: flume::Sender<HandlerAction>,
        shutdown: Arc<AtomicBool>,
    ) -> Result<String> {
        let url = if is_demo {
            url::Url::parse(WEBSOCKET_DEMO_URL).expect("Static WebSocket URL should be valid")
        } else {
            url::Url::parse(WEBSOCKET_URL).expect("Static WebSocket URL should be valid")
        };

        info!("Connecting to WebSocket: {}", url);
        let (ws_stream, _response) = connect_async(url).await?;
        info!("WebSocket connection established");

        // Notify that we've reconnected (or connected initially)
        let _ = event_sender.send_async(StreamEvent::Reconnected).await;

        let (mut write, mut read) = ws_stream.split();
        let (disconnect_sender, disconnect_receiver) = flume::bounded::<String>(1);

        // Task for reading from WebSocket
        let event_sender_clone = event_sender.clone();
        let disconnect_sender_read = disconnect_sender.clone();
        let shutdown_read = shutdown.clone();
        let read_task = tokio::spawn(async move {
            info!("AccountStreamer: Read task started, waiting for WebSocket messages");
            while let Some(message_result) = read.next().await {
                if shutdown_read.load(Ordering::SeqCst) {
                    debug!("Read task: shutdown requested");
                    break;
                }

                match message_result {
                    Ok(message) => {
                        let data = message.into_data();
                        // Log raw message receipt (truncate and mask sensitive data)
                        let raw_preview = String::from_utf8_lossy(&data);
                        let preview = if raw_preview.len() > 200 {
                            format!("{}...", &raw_preview[..200])
                        } else {
                            raw_preview.to_string()
                        };
                        // Mask account numbers in raw preview (pattern: digits followed by letters or 8+ char alphanumeric)
                        let masked_preview = mask_sensitive_data(&preview);
                        info!("AccountStreamer: Raw WebSocket message received ({} bytes): {}", data.len(), masked_preview);

                        match serde_json::from_slice::<AccountEvent>(&data) {
                            Ok(event) => {
                                // Log the event type for debugging
                                match &event {
                                    AccountEvent::AccountMessage(msg) => {
                                        debug!(
                                            "AccountStreamer: Received AccountMessage: {:?}",
                                            std::mem::discriminant(msg.as_ref())
                                        );
                                    }
                                    AccountEvent::StatusMessage(status) => {
                                        info!(
                                            "AccountStreamer: Received StatusMessage: action={}, status={}",
                                            status.action, status.status
                                        );
                                    }
                                    AccountEvent::HeartbeatResponse(hb) => {
                                        info!(
                                            "AccountStreamer: Received HeartbeatResponse: action={}, status={}, seq={}",
                                            hb.action, hb.status, hb.ws_sequence
                                        );
                                    }
                                    AccountEvent::ErrorMessage(err) => {
                                        warn!(
                                            "AccountStreamer: Received ErrorMessage: action={}, message={}",
                                            err.action, err.message
                                        );
                                    }
                                }

                                if event_sender_clone
                                    .send_async(StreamEvent::Account(event))
                                    .await
                                    .is_err()
                                {
                                    debug!("Event receiver dropped, stopping read task");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to parse WebSocket message: {} - raw: {}",
                                    e,
                                    String::from_utf8_lossy(&data)
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("WebSocket read error: {}", e);
                        let _ = disconnect_sender_read
                            .send_async(format!("WebSocket read error: {}", e))
                            .await;
                        break;
                    }
                }
            }
            let _ = disconnect_sender_read
                .send_async("WebSocket connection closed".to_string())
                .await;
        });

        // Task for writing to WebSocket
        let token_clone = token.to_string();
        let disconnect_sender_write = disconnect_sender.clone();
        let shutdown_write = shutdown.clone();
        let write_task = tokio::spawn(async move {
            info!("AccountStreamer: Write task started, waiting for actions");
            while let Ok(action) = action_receiver.recv_async().await {
                info!(
                    "AccountStreamer: Write task received {:?} action from channel",
                    action.action
                );
                if shutdown_write.load(Ordering::SeqCst) {
                    debug!("Write task: shutdown requested");
                    break;
                }

                let message = SubRequest {
                    auth_token: token_clone.clone(),
                    action: action.action,
                    value: action.value,
                    request_id: action.request_id,
                    source: SOURCE,
                };

                match serde_json::to_string(&message) {
                    Ok(json) => {
                        // Mask auth token in the logged JSON
                        let masked_json = json.replace(&token_clone, "***TOKEN***");
                        info!(
                            "AccountStreamer: Sending {:?} action over WebSocket: {}",
                            message.action, masked_json
                        );
                        let ws_message = Message::Text(json);
                        if let Err(e) = write.send(ws_message).await {
                            error!("WebSocket write error: {}", e);
                            let _ = disconnect_sender_write
                                .send_async(format!("WebSocket write error: {}", e))
                                .await;
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to serialize message: {}", e);
                    }
                }
            }
        });

        // Heartbeat task
        let sender_clone = action_sender.clone();
        let shutdown_heartbeat = shutdown.clone();
        let heartbeat_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS)).await;
                if shutdown_heartbeat.load(Ordering::SeqCst) {
                    debug!("Heartbeat task: shutdown requested");
                    break;
                }
                if sender_clone
                    .send_async(HandlerAction {
                        action: SubRequestAction::Heartbeat,
                        value: None,
                        request_id: 0, // Heartbeats use ws-sequence, not request-id
                    })
                    .await
                    .is_err()
                {
                    debug!("Action sender dropped, stopping heartbeat task");
                    break;
                }
            }
        });

        // Wait for disconnect signal
        let reason = disconnect_receiver
            .recv_async()
            .await
            .unwrap_or_else(|_| "Unknown disconnection reason".to_string());

        // Clean up tasks
        read_task.abort();
        write_task.abort();
        heartbeat_task.abort();

        Ok(reason)
    }

    /// Subscribe to account events for a specific account
    pub async fn subscribe_to_account<'a>(&self, account: &'a Account<'a>) {
        let account_number = account.inner.account.account_number.0.clone();

        // Store the account for re-subscription on reconnect
        {
            let mut state = self.state.lock().await;
            if !state.subscribed_accounts.contains(&account_number) {
                state.subscribed_accounts.push(account_number.clone());
            }
        }

        self.send(SubRequestAction::Connect, Some(vec![account_number]))
            .await;
    }

    /// Subscribe to account events by account number
    pub async fn subscribe_to_account_number(&self, account_number: &str) {
        info!(
            "AccountStreamer: Requesting subscription for account {}",
            mask_account(account_number)
        );

        // Store the account for re-subscription on reconnect
        {
            let mut state = self.state.lock().await;
            if !state.subscribed_accounts.contains(&account_number.to_string()) {
                state.subscribed_accounts.push(account_number.to_string());
            }
        }

        self.send(
            SubRequestAction::Connect,
            Some(vec![account_number.to_string()]),
        )
        .await;
    }

    /// Send an action to the streamer
    pub async fn send<T: Serialize + Send + Sync + 'static>(
        &self,
        action: SubRequestAction,
        value: Option<T>,
    ) {
        // Generate a unique request ID for this action
        let request_id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);

        if let Err(e) = self
            .action_sender
            .send_async(HandlerAction {
                action: action.clone(),
                value: value
                    .map(|inner| Box::new(inner) as Box<dyn erased_serde::Serialize + Send + Sync>),
                request_id,
            })
            .await
        {
            warn!("Failed to send action {:?}: {}", action, e);
        }
    }

    /// Close the streamer connection gracefully
    pub async fn close(&self) {
        info!("Closing AccountStreamer");
        self.shutdown.store(true, Ordering::SeqCst);

        // Send a dummy action to wake up the write task if it's blocked
        let _ = self
            .action_sender
            .send_async(HandlerAction {
                action: SubRequestAction::Heartbeat,
                value: None,
                request_id: 0,
            })
            .await;
    }

    /// Check if the streamer has been closed
    pub fn is_closed(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Get the next event from the streamer
    pub async fn get_event(&self) -> std::result::Result<StreamEvent, flume::RecvError> {
        self.event_receiver.recv_async().await
    }

    /// Try to get an event without blocking
    pub fn try_get_event(&self) -> std::result::Result<StreamEvent, flume::TryRecvError> {
        self.event_receiver.try_recv()
    }

    /// Get the list of currently subscribed accounts
    pub async fn subscribed_accounts(&self) -> Vec<String> {
        self.state.lock().await.subscribed_accounts.clone()
    }
}

impl TastyTrade {
    pub async fn create_account_streamer(&self) -> Result<AccountStreamer> {
        AccountStreamer::connect(self).await
    }

    pub async fn create_account_streamer_with_config(
        &self,
        config: AccountStreamerConfig,
    ) -> Result<AccountStreamer> {
        AccountStreamer::connect_with_config(self, config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::order::{InstrumentType, OrderStatus};
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_sub_request_action_serialization() {
        // Test serialization of all variants (they use kebab-case)
        assert_eq!(
            serde_json::to_string(&SubRequestAction::Heartbeat).unwrap(),
            "\"heartbeat\""
        );
        assert_eq!(
            serde_json::to_string(&SubRequestAction::Connect).unwrap(),
            "\"connect\""
        );
        assert_eq!(
            serde_json::to_string(&SubRequestAction::PublicWatchlistsSubscribe).unwrap(),
            "\"public-watchlists-subscribe\""
        );
        assert_eq!(
            serde_json::to_string(&SubRequestAction::QuoteAlertsSubscribe).unwrap(),
            "\"quote-alerts-subscribe\""
        );
        assert_eq!(
            serde_json::to_string(&SubRequestAction::UserMessageSubscribe).unwrap(),
            "\"user-message-subscribe\""
        );
    }

    #[test]
    fn test_sub_request_serialization() {
        let request = SubRequest {
            auth_token: "test_token".to_string(),
            action: SubRequestAction::Connect,
            value: Some("test_value"),
            request_id: 1,
            source: SOURCE,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Test kebab-case field names
        assert_eq!(parsed["auth-token"], "test_token");
        assert_eq!(parsed["action"], "connect"); // Action is also kebab-case
        assert_eq!(parsed["value"], "test_value");
        assert_eq!(parsed["request-id"], 1);
        assert_eq!(parsed["source"], SOURCE);
    }

    #[test]
    fn test_sub_request_serialization_no_value() {
        let request: SubRequest<String> = SubRequest {
            auth_token: "heartbeat_token".to_string(),
            action: SubRequestAction::Heartbeat,
            value: None,
            request_id: 0,
            source: SOURCE,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["auth-token"], "heartbeat_token");
        assert_eq!(parsed["action"], "heartbeat"); // Action is kebab-case
        assert!(parsed.get("value").is_none()); // value should be skipped when None
        assert_eq!(parsed["request-id"], 0);
        assert_eq!(parsed["source"], SOURCE);
    }

    #[test]
    fn test_status_message_deserialization() {
        let json = json!({
            "status": "ok",
            "action": "connect",
            "web-socket-session-id": "session123",
            "request-id": 42
        });

        let status: StatusMessage = serde_json::from_value(json).unwrap();
        assert_eq!(status.status, "ok");
        assert_eq!(status.action, "connect");
        assert_eq!(status.web_socket_session_id, "session123");
        assert_eq!(status.request_id, 42);
    }

    #[test]
    fn test_error_message_deserialization() {
        let json = json!({
            "status": "error",
            "action": "subscribe",
            "web-socket-session-id": "session456",
            "message": "Subscription failed"
        });

        let error: ErrorMessage = serde_json::from_value(json).unwrap();
        assert_eq!(error.status, "error");
        assert_eq!(error.action, "subscribe");
        assert_eq!(error.web_socket_session_id, "session456");
        assert_eq!(error.message, "Subscription failed");
    }

    #[test]
    fn test_account_message_order_variant() {
        // Create a minimal order record JSON
        let json = json!({
            "type": "Order",
            "data": {
                "id": 123456,
                "account-number": "ACC123",
                "time-in-force": "Day",
                "order-type": "Limit",
                "size": 100,
                "underlying-symbol": "AAPL",
                "price": "150.25",
                "price-effect": "Debit",
                "status": "Live",
                "cancellable": true,
                "editable": false,
                "edited": false
            }
        });

        let account_msg: AccountMessage = serde_json::from_value(json).unwrap();
        match account_msg {
            AccountMessage::Order(order) => {
                assert_eq!(order.id.0, 123456);
                assert_eq!(order.account_number.0, "ACC123");
                assert!(matches!(order.status, OrderStatus::Live));
            }
            _ => panic!("Expected Order variant"),
        }
    }

    #[test]
    fn test_account_message_balance_variant() {
        let json = json!({
            "type": "AccountBalance",
            "data": {
                "account-number": "ACC456",
                "cash-balance": "10000.50",
                "long-equity-value": "25000.75",
                "short-equity-value": "0.00",
                "long-derivative-value": "5000.25",
                "short-derivative-value": "1000.00",
                "long-futures-value": "0.00",
                "short-futures-value": "0.00",
                "long-futures-derivative-value": "0.00",
                "short-futures-derivative-value": "0.00",
                "long-margineable-value": "30000.00",
                "short-margineable-value": "1000.00",
                "margin-equity": "39000.50",
                "equity-buying-power": "78000.00",
                "derivative-buying-power": "39000.50",
                "day-trading-buying-power": "156000.00",
                "futures-margin-requirement": "0.00",
                "available-trading-funds": "39000.50",
                "maintenance-requirement": "7500.00",
                "maintenance-call-value": "0.00",
                "reg-t-call-value": "0.00",
                "day-trading-call-value": "0.00",
                "day-equity-call-value": "0.00",
                "net-liquidating-value": "39000.50",
                "cash-available-to-withdraw": "10000.50",
                "day-trade-excess": "148500.00",
                "pending-cash": "0.00",
                "pending-cash-effect": "None",
                "pending-margin-interest": "0.00",
                "effective-cryptocurrency-buying-power": "0.00",
                "updated-at": "2023-01-01T12:00:00Z"
            }
        });

        let account_msg: AccountMessage = serde_json::from_value(json).unwrap();
        match account_msg {
            AccountMessage::AccountBalance(balance) => {
                assert_eq!(balance.account_number.0, "ACC456");
                assert_eq!(balance.cash_balance, Decimal::from_str("10000.50").unwrap());
                assert_eq!(
                    balance.net_liquidating_value,
                    Decimal::from_str("39000.50").unwrap()
                );
            }
            _ => panic!("Expected AccountBalance variant"),
        }
    }

    #[test]
    fn test_account_message_position_variant() {
        let json = json!({
            "type": "CurrentPosition",
            "data": {
                "account-number": "ACC789",
                "symbol": "SPY",
                "instrument-type": "Equity",
                "underlying-symbol": "SPY",
                "quantity": "100.00",
                "quantity-direction": "Long",
                "close-price": "450.25",
                "average-open-price": "445.50",
                "multiplier": 1.0,
                "cost-effect": "Debit",
                "is-suppressed": false,
                "is-frozen": false,
                "restricted-quantity": 0.0,
                "realized-day-gain": "475.00",
                "realized-today": "475.00",
                "created-at": "2023-01-01T10:00:00Z",
                "updated-at": "2023-01-01T16:00:00Z"
            }
        });

        let account_msg: AccountMessage = serde_json::from_value(json).unwrap();
        match account_msg {
            AccountMessage::CurrentPosition(position) => {
                assert_eq!(position.account_number.0, "ACC789");
                assert_eq!(position.symbol.0, "SPY");
                assert_eq!(position.quantity, Decimal::from_str("100.00").unwrap());
                assert!(matches!(position.instrument_type, InstrumentType::Equity));
            }
            _ => panic!("Expected CurrentPosition variant"),
        }
    }

    #[test]
    fn test_account_message_other_variants() {
        // Test OrderChain variant (no data needed)
        let json = json!({
            "type": "OrderChain"
        });
        let account_msg: AccountMessage = serde_json::from_value(json).unwrap();
        assert!(matches!(account_msg, AccountMessage::OrderChain));

        // Test ExternalTransaction variant (no data needed)
        let json = json!({
            "type": "ExternalTransaction"
        });
        let account_msg: AccountMessage = serde_json::from_value(json).unwrap();
        assert!(matches!(account_msg, AccountMessage::ExternalTransaction));
    }

    #[test]
    fn test_account_event_untagged_deserialization() {
        // Test Status Message case
        let status_json = json!({
            "status": "ok",
            "action": "heartbeat",
            "web-socket-session-id": "session1",
            "request-id": 1
        });

        let event: AccountEvent = serde_json::from_value(status_json).unwrap();
        match event {
            AccountEvent::StatusMessage(status) => {
                assert_eq!(status.status, "ok");
                assert_eq!(status.action, "heartbeat");
            }
            _ => panic!("Expected StatusMessage variant"),
        }

        // Test Error Message case
        let error_json = json!({
            "status": "error",
            "action": "connect",
            "web-socket-session-id": "session2",
            "message": "Authentication failed"
        });

        let event: AccountEvent = serde_json::from_value(error_json).unwrap();
        match event {
            AccountEvent::ErrorMessage(error) => {
                assert_eq!(error.status, "error");
                assert_eq!(error.message, "Authentication failed");
            }
            _ => panic!("Expected ErrorMessage variant"),
        }

        // Test Account Message case
        let account_json = json!({
            "type": "OrderChain"
        });

        let event: AccountEvent = serde_json::from_value(account_json).unwrap();
        match event {
            AccountEvent::AccountMessage(msg) => {
                assert!(matches!(msg.as_ref(), AccountMessage::OrderChain));
            }
            _ => panic!("Expected AccountMessage variant"),
        }
    }

    #[test]
    fn test_account_event_disambiguation() {
        // Test that untagged enum correctly disambiguates between variants
        // StatusMessage has request_id but no message
        let status_json = json!({
            "status": "connected",
            "action": "connect",
            "web-socket-session-id": "sess1",
            "request-id": 123
        });

        let event: AccountEvent = serde_json::from_value(status_json).unwrap();
        assert!(matches!(event, AccountEvent::StatusMessage(_)));

        // ErrorMessage has message but no request_id
        let error_json = json!({
            "status": "error",
            "action": "subscribe",
            "web-socket-session-id": "sess2",
            "message": "Invalid account"
        });

        let event: AccountEvent = serde_json::from_value(error_json).unwrap();
        assert!(matches!(event, AccountEvent::ErrorMessage(_)));

        // AccountMessage has type field
        let account_json = json!({
            "type": "Order",
            "data": {
                "id": 789,
                "account-number": "TEST123",
                "time-in-force": "GTC",
                "order-type": "Market",
                "size": 50,
                "underlying-symbol": "TEST",
                "price": "100.00",
                "price-effect": "Credit",
                "status": "Filled",
                "cancellable": false,
                "editable": false,
                "edited": false
            }
        });

        let event: AccountEvent = serde_json::from_value(account_json).unwrap();
        assert!(matches!(event, AccountEvent::AccountMessage(_)));
    }

    #[test]
    fn test_heartbeat_response_deserialization() {
        // Test heartbeat response (uses ws-sequence instead of request-id)
        let json = json!({
            "web-socket-session-id": "8fb26a40-92be-94ac-bdb3-b1c5b1e38ede",
            "action": "heartbeat",
            "status": "ok",
            "ws-sequence": 3
        });

        let event: AccountEvent = serde_json::from_value(json).unwrap();
        match event {
            AccountEvent::HeartbeatResponse(heartbeat) => {
                assert_eq!(heartbeat.action, "heartbeat");
                assert_eq!(heartbeat.status, "ok");
                assert_eq!(heartbeat.ws_sequence, 3);
                assert_eq!(
                    heartbeat.web_socket_session_id,
                    "8fb26a40-92be-94ac-bdb3-b1c5b1e38ede"
                );
            }
            _ => panic!("Expected HeartbeatResponse variant"),
        }
    }

    #[test]
    fn test_heartbeat_response_direct_deserialization() {
        // Test direct deserialization of HeartbeatResponse struct
        let json = json!({
            "status": "ok",
            "action": "heartbeat",
            "web-socket-session-id": "session123",
            "ws-sequence": 42
        });

        let heartbeat: HeartbeatResponse = serde_json::from_value(json).unwrap();
        assert_eq!(heartbeat.status, "ok");
        assert_eq!(heartbeat.action, "heartbeat");
        assert_eq!(heartbeat.web_socket_session_id, "session123");
        assert_eq!(heartbeat.ws_sequence, 42);
    }

    #[test]
    fn test_streamer_config_default() {
        let config = AccountStreamerConfig::default();
        assert!(config.auto_reconnect);
        assert_eq!(config.max_reconnect_attempts, MAX_RECONNECT_ATTEMPTS);
        assert_eq!(config.initial_backoff_ms, INITIAL_BACKOFF_MS);
        assert_eq!(config.max_backoff_ms, MAX_BACKOFF_MS);
    }

    #[test]
    fn test_streamer_config_custom() {
        let config = AccountStreamerConfig {
            auto_reconnect: false,
            max_reconnect_attempts: 5,
            initial_backoff_ms: 500,
            max_backoff_ms: 10000,
        };
        assert!(!config.auto_reconnect);
        assert_eq!(config.max_reconnect_attempts, 5);
        assert_eq!(config.initial_backoff_ms, 500);
        assert_eq!(config.max_backoff_ms, 10000);
    }

    #[test]
    fn test_stream_event_variants() {
        // Test Disconnected variant
        let event = StreamEvent::Disconnected {
            reason: "Connection lost".to_string(),
        };
        match event {
            StreamEvent::Disconnected { reason } => {
                assert_eq!(reason, "Connection lost");
            }
            _ => panic!("Expected Disconnected variant"),
        }

        // Test Reconnecting variant
        let event = StreamEvent::Reconnecting {
            attempt: 3,
            max_attempts: 10,
        };
        match event {
            StreamEvent::Reconnecting {
                attempt,
                max_attempts,
            } => {
                assert_eq!(attempt, 3);
                assert_eq!(max_attempts, 10);
            }
            _ => panic!("Expected Reconnecting variant"),
        }

        // Test Reconnected variant
        let event = StreamEvent::Reconnected;
        assert!(matches!(event, StreamEvent::Reconnected));

        // Test Closed variant
        let event = StreamEvent::Closed {
            reason: "Max retries reached".to_string(),
        };
        match event {
            StreamEvent::Closed { reason } => {
                assert_eq!(reason, "Max retries reached");
            }
            _ => panic!("Expected Closed variant"),
        }
    }

    // Note: We can't easily test the full AccountStreamer functionality without
    // setting up actual WebSocket connections, which would require integration tests
    // marked with #[ignore]. The core streaming functionality involves async spawned
    // tasks and real network connections.
}
