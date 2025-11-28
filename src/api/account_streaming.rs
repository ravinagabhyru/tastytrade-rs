use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    api::accounts::{Account, Balance},
    api::base::Result,
    TastyTrade,
};

use super::{order::LiveOrderRecord, position::BriefPosition};

static WEBSOCKET_DEMO_URL: &str = "wss://streamer.cert.tastyworks.com";
static WEBSOCKET_URL: &str = "wss://streamer.tastyworks.com";

#[derive(Debug, Serialize)]
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
    value: Option<T>,
}

pub struct HandlerAction {
    action: SubRequestAction,
    value: Option<Box<dyn erased_serde::Serialize + Send + Sync>>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum AccountMessage {
    Order(LiveOrderRecord),
    AccountBalance(Box<Balance>),
    CurrentPosition(Box<BriefPosition>),
    OrderChain,
    ExternalTransaction,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct StatusMessage {
    pub status: String,
    pub action: String,
    pub web_socket_session_id: String,
    pub request_id: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct ErrorMessage {
    pub status: String,
    pub action: String,
    pub web_socket_session_id: String,
    pub message: String,
}

//#[allow(clippy::large_enum_variant)]
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum AccountEvent {
    ErrorMessage(ErrorMessage),
    StatusMessage(StatusMessage),
    AccountMessage(Box<AccountMessage>),
}

#[derive(Debug)]
pub struct AccountStreamer {
    pub event_receiver: flume::Receiver<AccountEvent>,
    pub action_sender: flume::Sender<HandlerAction>,
}

impl AccountStreamer {
    pub async fn connect(tasty: &TastyTrade) -> Result<AccountStreamer> {
        // Capture the current auth header value for websocket auth
        let token = {
            // best-effort snapshot; websocket does not auto-refresh this value
            // if the access token expires, the streaming connection may need reestablishing
            tasty.auth_state.read().await.auth_header()
        };
        let (event_sender, event_receiver) = flume::unbounded();
        let (action_sender, action_receiver): (
            flume::Sender<HandlerAction>,
            flume::Receiver<HandlerAction>,
        ) = flume::unbounded();

        let url = if tasty.demo {
            url::Url::parse(WEBSOCKET_DEMO_URL).unwrap()
        } else {
            url::Url::parse(WEBSOCKET_URL).unwrap()
        };

        let (ws_stream, _response) = connect_async(url).await?;
        // let hello = ws_stream.try_next().await?;
        // if let Some(msg) = hello {
        //     match serde_json::from_slice(&msg.into_data())? {
        //         SubMessage::ErrorMessage(_) => return Err(ConnectionClosed.into()), // Perhaps retry on our own?
        //         SubMessage::StatusMessage(_) => {}
        //         _ => unreachable!(),
        //     }
        // } else {
        //     return Err(ConnectionClosed.into());
        // }

        let (mut write, mut read) = ws_stream.split();

        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                let data = message.unwrap().into_data();
                //println!("{:?}", String::from_utf8_lossy(&data));
                let data: AccountEvent = serde_json::from_slice(&data).unwrap();
                event_sender.send_async(data).await.unwrap();
            }
        });

        let token_clone = token.clone();
        tokio::spawn(async move {
            while let Ok(action) = action_receiver.recv_async().await {
                let message = SubRequest {
                    auth_token: token_clone.clone(),
                    action: action.action,
                    value: action.value,
                };
                let message = serde_json::to_string(&message).unwrap();

                //println!("{message:?}");

                let message = Message::Text(message);

                if write.send(message).await.is_err() {
                    // TODO: send message informing user of disconnection
                    break;
                }
            }
        });

        let sender_clone = action_sender.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                if sender_clone
                    .send_async(HandlerAction {
                        action: SubRequestAction::Heartbeat,
                        value: None,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok(Self {
            event_receiver,
            action_sender,
        })
    }

    pub async fn subscribe_to_account<'a>(&self, account: &'a Account<'a>) {
        self.send(
            SubRequestAction::Connect,
            Some(vec![account.inner.account.account_number.clone()]),
        )
        .await;
    }

    pub async fn send<T: Serialize + Send + Sync + 'static>(
        &self,
        action: SubRequestAction,
        value: Option<T>,
    ) {
        self.action_sender
            .send_async(HandlerAction {
                action,
                value: value
                    .map(|inner| Box::new(inner) as Box<dyn erased_serde::Serialize + Send + Sync>),
            })
            .await
            .unwrap();
    }

    // pub async fn close(&self) {}

    pub async fn get_event(&self) -> std::result::Result<AccountEvent, flume::RecvError> {
        self.event_receiver.recv_async().await
    }
}

impl TastyTrade {
    pub async fn create_account_streamer(&self) -> Result<AccountStreamer> {
        AccountStreamer::connect(self).await
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
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Test kebab-case field names
        assert_eq!(parsed["auth-token"], "test_token");
        assert_eq!(parsed["action"], "connect"); // Action is also kebab-case
        assert_eq!(parsed["value"], "test_value");
    }

    #[test]
    fn test_sub_request_serialization_no_value() {
        let request: SubRequest<String> = SubRequest {
            auth_token: "heartbeat_token".to_string(),
            action: SubRequestAction::Heartbeat,
            value: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["auth-token"], "heartbeat_token");
        assert_eq!(parsed["action"], "heartbeat"); // Action is kebab-case
        assert!(parsed["value"].is_null());
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

    // Note: We can't easily test the full AccountStreamer functionality without
    // setting up actual WebSocket connections, which would require integration tests
    // marked with #[ignore]. The core streaming functionality involves async spawned
    // tasks and real network connections.
}
