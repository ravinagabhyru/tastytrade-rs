use derive_builder::Builder;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::api::accounts::AccountNumber;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PriceEffect {
    Debit,
    Credit,
    None,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    #[serde(rename = "Buy to Open")]
    BuyToOpen,
    #[serde(rename = "Sell to Open")]
    SellToOpen,
    #[serde(rename = "Buy to Close")]
    BuyToClose,
    #[serde(rename = "Sell to Close")]
    SellToClose,
    Sell,
    Buy,
}

impl Action {
    pub fn as_api_str(&self) -> &str {
        match self {
            Action::BuyToOpen => "Buy to Open",
            Action::SellToOpen => "Sell to Open",
            Action::BuyToClose => "Buy to Close",
            Action::SellToClose => "Sell to Close",
            Action::Sell => "Sell",
            Action::Buy => "Buy",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InstrumentType {
    Equity,
    #[serde(rename = "Equity Option")]
    EquityOption,
    #[serde(rename = "Equity Offering")]
    EquityOffering,
    Future,
    #[serde(rename = "Future Option")]
    FutureOption,
    Cryptocurrency,
    Bond,
    Index,
    Warrant,
    #[serde(other)]
    Unknown,
}

impl InstrumentType {
    pub fn as_api_str(&self) -> &str {
        match self {
            InstrumentType::Equity => "Equity",
            InstrumentType::EquityOption => "Equity Option",
            InstrumentType::EquityOffering => "Equity Offering",
            InstrumentType::Future => "Future",
            InstrumentType::FutureOption => "Future Option",
            InstrumentType::Cryptocurrency => "Cryptocurrency",
            InstrumentType::Bond => "Bond",
            InstrumentType::Index => "Index",
            InstrumentType::Warrant => "Warrant",
            InstrumentType::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderType {
    Limit,
    Market,
    #[serde(rename = "Marketable Limit")]
    MarketableLimit,
    Stop,
    #[serde(rename = "Stop Limit")]
    StopLimit,
    #[serde(rename = "Notional Market")]
    NotionalMarket,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TimeInForce {
    Day,
    GTC,
    GTD,
    Ext,
    #[serde(rename = "GTC Ext")]
    GTCExt,
    IOC,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderStatus {
    Received,
    Routed,
    #[serde(rename = "In Flight")]
    InFlight,
    Live,
    #[serde(rename = "Cancel Requested")]
    CancelRequested,
    #[serde(rename = "Replace Requested")]
    ReplaceRequested,
    Contingent,
    Filled,
    Cancelled,
    Expired,
    Rejected,
    Removed,
    #[serde(rename = "Partially Removed")]
    PartiallyRemoved,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct Symbol(pub String);

impl<T: AsRef<str>> From<T> for Symbol {
    fn from(value: T) -> Self {
        Self(value.as_ref().to_owned())
    }
}

pub trait AsSymbol {
    fn as_symbol(&self) -> Symbol;
}

impl<T: AsRef<str>> AsSymbol for T {
    fn as_symbol(&self) -> Symbol {
        Symbol(self.as_ref().to_owned())
    }
}

impl AsSymbol for Symbol {
    fn as_symbol(&self) -> Symbol {
        self.clone()
    }
}

impl AsSymbol for &Symbol {
    fn as_symbol(&self) -> Symbol {
        (*self).clone()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct OrderId(pub u64);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LiveOrderRecord {
    pub id: OrderId,
    pub account_number: AccountNumber,
    pub time_in_force: TimeInForce,
    pub order_type: OrderType,
    pub size: u64,
    pub underlying_symbol: Symbol,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub price: Decimal,
    pub price_effect: PriceEffect,
    pub status: OrderStatus,
    pub cancellable: bool,
    pub editable: bool,
    pub edited: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LiveOrderLeg {
    pub instrument_type: InstrumentType,
    pub symbol: Symbol,
    pub quantity: u64,
    pub remaining_quantity: u64,
    pub action: Action,
    pub fills: Vec<String>,
}

#[derive(Builder, Serialize)]
#[serde(rename_all = "kebab-case")]
#[builder(setter(into, strip_option))]
pub struct Order {
    time_in_force: TimeInForce,
    order_type: OrderType,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    price: Option<Decimal>,
    price_effect: PriceEffect,
    legs: Vec<OrderLeg>,
}

impl Default for Order {
    fn default() -> Self {
        Self {
            time_in_force: TimeInForce::Day,
            order_type: OrderType::Market,
            price: None,
            price_effect: PriceEffect::None,
            legs: Vec::new(),
        }
    }
}

#[derive(Builder, Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
#[builder(setter(into))]
pub struct OrderLeg {
    instrument_type: InstrumentType,
    symbol: Symbol,
    #[serde(with = "rust_decimal::serde::float")]
    quantity: Decimal,
    action: Action,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OrderPlacedResult {
    pub order: LiveOrderRecord,
    pub warnings: Vec<Warning>,
    pub buying_power_effect: BuyingPowerEffect,
    pub fee_calculation: FeeCalculation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DryRunResult {
    pub order: DryRunRecord,
    pub warnings: Vec<Warning>,
    pub buying_power_effect: BuyingPowerEffect,
    pub fee_calculation: FeeCalculation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DryRunRecord {
    pub account_number: AccountNumber,
    pub time_in_force: TimeInForce,
    pub order_type: OrderType,
    pub size: u64,
    pub underlying_symbol: Symbol,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub price: Decimal,
    pub price_effect: PriceEffect,
    pub status: OrderStatus,
    pub cancellable: bool,
    pub editable: bool,
    pub edited: bool,
    pub legs: Vec<OrderLeg>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuyingPowerEffect {
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub change_in_margin_requirement: Decimal,
    pub change_in_margin_requirement_effect: PriceEffect,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub change_in_buying_power: Decimal,
    pub change_in_buying_power_effect: PriceEffect,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub current_buying_power: Decimal,
    pub current_buying_power_effect: PriceEffect,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub impact: Decimal,
    pub effect: PriceEffect,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FeeCalculation {
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub total_fees: Decimal,
    pub total_fees_effect: PriceEffect,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Warning {}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_symbol_from_str() {
        let symbol = Symbol::from("AAPL");
        assert_eq!(symbol.0, "AAPL");
    }

    #[test]
    fn test_symbol_from_string() {
        let symbol = Symbol::from("SPY".to_string());
        assert_eq!(symbol.0, "SPY");
    }

    #[test]
    fn test_as_symbol_for_str() {
        let symbol = "MSFT".as_symbol();
        assert_eq!(symbol.0, "MSFT");
    }

    #[test]
    fn test_as_symbol_for_string() {
        let symbol = "TSLA".to_string().as_symbol();
        assert_eq!(symbol.0, "TSLA");
    }

    #[test]
    fn test_as_symbol_for_symbol() {
        let original = Symbol::from("GOOGL");
        let symbol = original.as_symbol();
        assert_eq!(symbol.0, "GOOGL");
    }

    #[test]
    fn test_as_symbol_for_symbol_ref() {
        let original = Symbol::from("NVDA");
        let symbol = (&original).as_symbol();
        assert_eq!(symbol.0, "NVDA");
    }

    #[test]
    fn test_action_serde() {
        // Test all renamed variants
        assert_eq!(
            serde_json::to_string(&Action::BuyToOpen).unwrap(),
            "\"Buy to Open\""
        );
        assert_eq!(
            serde_json::to_string(&Action::SellToOpen).unwrap(),
            "\"Sell to Open\""
        );
        assert_eq!(
            serde_json::to_string(&Action::BuyToClose).unwrap(),
            "\"Buy to Close\""
        );
        assert_eq!(
            serde_json::to_string(&Action::SellToClose).unwrap(),
            "\"Sell to Close\""
        );
        assert_eq!(serde_json::to_string(&Action::Buy).unwrap(), "\"Buy\"");
        assert_eq!(serde_json::to_string(&Action::Sell).unwrap(), "\"Sell\"");

        // Test deserialization
        assert!(matches!(
            serde_json::from_str::<Action>("\"Buy to Open\"").unwrap(),
            Action::BuyToOpen
        ));
        assert!(matches!(
            serde_json::from_str::<Action>("\"Sell to Close\"").unwrap(),
            Action::SellToClose
        ));
    }

    #[test]
    fn test_instrument_type_serde() {
        assert_eq!(
            serde_json::to_string(&InstrumentType::Equity).unwrap(),
            "\"Equity\""
        );
        assert_eq!(
            serde_json::to_string(&InstrumentType::EquityOption).unwrap(),
            "\"Equity Option\""
        );
        assert_eq!(
            serde_json::to_string(&InstrumentType::EquityOffering).unwrap(),
            "\"Equity Offering\""
        );
        assert_eq!(
            serde_json::to_string(&InstrumentType::Future).unwrap(),
            "\"Future\""
        );
        assert_eq!(
            serde_json::to_string(&InstrumentType::FutureOption).unwrap(),
            "\"Future Option\""
        );
        assert_eq!(
            serde_json::to_string(&InstrumentType::Cryptocurrency).unwrap(),
            "\"Cryptocurrency\""
        );
        assert_eq!(
            serde_json::to_string(&InstrumentType::Bond).unwrap(),
            "\"Bond\""
        );
        assert_eq!(
            serde_json::to_string(&InstrumentType::Index).unwrap(),
            "\"Index\""
        );
        assert_eq!(
            serde_json::to_string(&InstrumentType::Warrant).unwrap(),
            "\"Warrant\""
        );

        // Test deserialization
        assert!(matches!(
            serde_json::from_str::<InstrumentType>("\"Equity Option\"").unwrap(),
            InstrumentType::EquityOption
        ));
        assert!(matches!(
            serde_json::from_str::<InstrumentType>("\"Future Option\"").unwrap(),
            InstrumentType::FutureOption
        ));
        assert!(matches!(
            serde_json::from_str::<InstrumentType>("\"Bond\"").unwrap(),
            InstrumentType::Bond
        ));
        assert!(matches!(
            serde_json::from_str::<InstrumentType>("\"Index\"").unwrap(),
            InstrumentType::Index
        ));
        assert!(matches!(
            serde_json::from_str::<InstrumentType>("\"Warrant\"").unwrap(),
            InstrumentType::Warrant
        ));

        // Test unknown variant
        assert!(matches!(
            serde_json::from_str::<InstrumentType>("\"SomeUnknownType\"").unwrap(),
            InstrumentType::Unknown
        ));
    }

    #[test]
    fn test_instrument_type_as_api_str() {
        assert_eq!(InstrumentType::Equity.as_api_str(), "Equity");
        assert_eq!(InstrumentType::EquityOption.as_api_str(), "Equity Option");
        assert_eq!(InstrumentType::EquityOffering.as_api_str(), "Equity Offering");
        assert_eq!(InstrumentType::Future.as_api_str(), "Future");
        assert_eq!(InstrumentType::FutureOption.as_api_str(), "Future Option");
        assert_eq!(InstrumentType::Cryptocurrency.as_api_str(), "Cryptocurrency");
        assert_eq!(InstrumentType::Bond.as_api_str(), "Bond");
        assert_eq!(InstrumentType::Index.as_api_str(), "Index");
        assert_eq!(InstrumentType::Warrant.as_api_str(), "Warrant");
        assert_eq!(InstrumentType::Unknown.as_api_str(), "Unknown");
    }

    #[test]
    fn test_order_type_serde() {
        assert_eq!(
            serde_json::to_string(&OrderType::Limit).unwrap(),
            "\"Limit\""
        );
        assert_eq!(
            serde_json::to_string(&OrderType::Market).unwrap(),
            "\"Market\""
        );
        assert_eq!(
            serde_json::to_string(&OrderType::MarketableLimit).unwrap(),
            "\"Marketable Limit\""
        );
        assert_eq!(serde_json::to_string(&OrderType::Stop).unwrap(), "\"Stop\"");
        assert_eq!(
            serde_json::to_string(&OrderType::StopLimit).unwrap(),
            "\"Stop Limit\""
        );
        assert_eq!(
            serde_json::to_string(&OrderType::NotionalMarket).unwrap(),
            "\"Notional Market\""
        );

        // Test deserialization
        assert!(matches!(
            serde_json::from_str::<OrderType>("\"Marketable Limit\"").unwrap(),
            OrderType::MarketableLimit
        ));
        assert!(matches!(
            serde_json::from_str::<OrderType>("\"Stop Limit\"").unwrap(),
            OrderType::StopLimit
        ));
    }

    #[test]
    fn test_time_in_force_serde() {
        assert_eq!(serde_json::to_string(&TimeInForce::Day).unwrap(), "\"Day\"");
        assert_eq!(serde_json::to_string(&TimeInForce::GTC).unwrap(), "\"GTC\"");
        assert_eq!(
            serde_json::to_string(&TimeInForce::GTCExt).unwrap(),
            "\"GTC Ext\""
        );
        assert_eq!(serde_json::to_string(&TimeInForce::IOC).unwrap(), "\"IOC\"");

        // Test deserialization
        assert!(matches!(
            serde_json::from_str::<TimeInForce>("\"GTC Ext\"").unwrap(),
            TimeInForce::GTCExt
        ));
    }

    #[test]
    fn test_order_status_serde() {
        assert_eq!(
            serde_json::to_string(&OrderStatus::Received).unwrap(),
            "\"Received\""
        );
        assert_eq!(
            serde_json::to_string(&OrderStatus::InFlight).unwrap(),
            "\"In Flight\""
        );
        assert_eq!(
            serde_json::to_string(&OrderStatus::CancelRequested).unwrap(),
            "\"Cancel Requested\""
        );
        assert_eq!(
            serde_json::to_string(&OrderStatus::ReplaceRequested).unwrap(),
            "\"Replace Requested\""
        );
        assert_eq!(
            serde_json::to_string(&OrderStatus::PartiallyRemoved).unwrap(),
            "\"Partially Removed\""
        );

        // Test deserialization
        assert!(matches!(
            serde_json::from_str::<OrderStatus>("\"In Flight\"").unwrap(),
            OrderStatus::InFlight
        ));
        assert!(matches!(
            serde_json::from_str::<OrderStatus>("\"Cancel Requested\"").unwrap(),
            OrderStatus::CancelRequested
        ));
    }

    #[test]
    fn test_order_leg_quantity_float_serialization() {
        let leg = OrderLeg {
            instrument_type: InstrumentType::Equity,
            symbol: Symbol::from("AAPL"),
            quantity: Decimal::from_str("100.5").unwrap(),
            action: Action::Buy,
        };

        let json = serde_json::to_string(&leg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify quantity is serialized as a float, not a string
        assert_eq!(parsed["quantity"], 100.5);
    }

    #[test]
    fn test_order_price_arbitrary_precision_serialization() {
        let order = OrderBuilder::default()
            .time_in_force(TimeInForce::Day)
            .order_type(OrderType::Limit)
            .price(Decimal::from_str("123.456789").unwrap())
            .price_effect(PriceEffect::Debit)
            .legs(vec![])
            .build()
            .unwrap();

        let json = serde_json::to_string(&order).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify price is serialized correctly (may be string or number based on serde config)
        // The important part is that the precision is preserved
        let price_val = parsed["price"]
            .as_str()
            .map(|s| Decimal::from_str(s).unwrap())
            .or_else(|| {
                parsed["price"]
                    .as_f64()
                    .map(Decimal::try_from)
                    .map(|r| r.unwrap())
            })
            .unwrap();
        assert_eq!(price_val, Decimal::from_str("123.456789").unwrap());
    }

    #[test]
    fn test_live_order_record_deserialization() {
        let json = json!({
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
        });

        let record: LiveOrderRecord = serde_json::from_value(json).unwrap();
        assert_eq!(record.id.0, 123456);
        assert_eq!(record.account_number.0, "ACC123");
        assert!(matches!(record.time_in_force, TimeInForce::Day));
        assert!(matches!(record.order_type, OrderType::Limit));
        assert_eq!(record.size, 100);
        assert_eq!(record.underlying_symbol.0, "AAPL");
        assert_eq!(record.price, Decimal::from_str("150.25").unwrap());
        assert!(matches!(record.price_effect, PriceEffect::Debit));
        assert!(matches!(record.status, OrderStatus::Live));
        assert!(record.cancellable);
        assert!(!record.editable);
        assert!(!record.edited);
    }

    #[test]
    fn test_dry_run_record_deserialization() {
        let json = json!({
            "account-number": "ACC456",
            "time-in-force": "GTC",
            "order-type": "Market",
            "size": 50,
            "underlying-symbol": "SPY",
            "price": "420.00",
            "price-effect": "Credit",
            "status": "Received",
            "cancellable": false,
            "editable": true,
            "edited": true,
            "legs": []
        });

        let record: DryRunRecord = serde_json::from_value(json).unwrap();
        assert_eq!(record.account_number.0, "ACC456");
        assert!(matches!(record.time_in_force, TimeInForce::GTC));
        assert!(matches!(record.order_type, OrderType::Market));
        assert_eq!(record.size, 50);
        assert_eq!(record.underlying_symbol.0, "SPY");
        assert_eq!(record.price, Decimal::from_str("420.00").unwrap());
        assert!(matches!(record.price_effect, PriceEffect::Credit));
        assert!(matches!(record.status, OrderStatus::Received));
        assert!(!record.cancellable);
        assert!(record.editable);
        assert!(record.edited);
        assert!(record.legs.is_empty());
    }

    #[test]
    fn test_order_builder_equity_order() {
        let leg = OrderLegBuilder::default()
            .instrument_type(InstrumentType::Equity)
            .symbol(Symbol::from("TSLA"))
            .quantity(Decimal::from(100))
            .action(Action::Buy)
            .build()
            .unwrap();

        let order = OrderBuilder::default()
            .time_in_force(TimeInForce::Day)
            .order_type(OrderType::Market)
            .price(Decimal::ZERO)
            .price_effect(PriceEffect::Debit)
            .legs(vec![leg])
            .build()
            .unwrap();

        // Verify the order was built correctly
        let json = serde_json::to_string(&order).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["time-in-force"], "Day");
        assert_eq!(parsed["order-type"], "Market");
        // Price may be serialized as number or string - check the value
        let price_val = parsed["price"]
            .as_str()
            .map(|s| Decimal::from_str(s).unwrap())
            .or_else(|| {
                parsed["price"]
                    .as_f64()
                    .map(Decimal::try_from)
                    .map(|r| r.unwrap())
            })
            .unwrap();
        assert_eq!(price_val, Decimal::ZERO);
        assert_eq!(parsed["price-effect"], "Debit");
        assert_eq!(parsed["legs"].as_array().unwrap().len(), 1);

        let leg_json = &parsed["legs"][0];
        assert_eq!(leg_json["instrument-type"], "Equity");
        assert_eq!(leg_json["symbol"], "TSLA");
        assert_eq!(leg_json["quantity"], 100.0);
        assert_eq!(leg_json["action"], "Buy");
    }

    #[test]
    fn test_order_builder_options_order() {
        let leg = OrderLegBuilder::default()
            .instrument_type(InstrumentType::EquityOption)
            .symbol(Symbol::from("AAPL  240315C00185000"))
            .quantity(Decimal::from(1))
            .action(Action::BuyToOpen)
            .build()
            .unwrap();

        let order = OrderBuilder::default()
            .time_in_force(TimeInForce::GTC)
            .order_type(OrderType::Limit)
            .price(Decimal::from_str("5.50").unwrap())
            .price_effect(PriceEffect::Debit)
            .legs(vec![leg])
            .build()
            .unwrap();

        let json = serde_json::to_string(&order).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["time-in-force"], "GTC");
        assert_eq!(parsed["order-type"], "Limit");
        // Price may be serialized as number or string - check the value
        let price_val = parsed["price"]
            .as_str()
            .map(|s| Decimal::from_str(s).unwrap())
            .or_else(|| {
                parsed["price"]
                    .as_f64()
                    .map(Decimal::try_from)
                    .map(|r| r.unwrap())
            })
            .unwrap();
        assert_eq!(price_val, Decimal::from_str("5.50").unwrap());
        assert_eq!(parsed["price-effect"], "Debit");

        let leg_json = &parsed["legs"][0];
        assert_eq!(leg_json["instrument-type"], "Equity Option");
        assert_eq!(leg_json["symbol"], "AAPL  240315C00185000");
        assert_eq!(leg_json["quantity"], 1.0);
        assert_eq!(leg_json["action"], "Buy to Open");
    }

    #[test]
    #[should_panic]
    fn test_order_builder_missing_required_fields() {
        // This should panic because we're missing required fields
        OrderBuilder::default()
            .time_in_force(TimeInForce::Day)
            // Missing order_type, price, price_effect, legs
            .build()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn test_order_leg_builder_missing_required_fields() {
        // This should panic because we're missing required fields
        OrderLegBuilder::default()
            .symbol(Symbol::from("AAPL"))
            // Missing instrument_type, quantity, action
            .build()
            .unwrap();
    }

    #[test]
    fn test_symbol_ordering_and_equality() {
        let symbol1 = Symbol::from("AAPL");
        let symbol2 = Symbol::from("AAPL");
        let symbol3 = Symbol::from("MSFT");

        assert_eq!(symbol1, symbol2);
        assert_ne!(symbol1, symbol3);
        assert!(symbol1 < symbol3); // AAPL < MSFT alphabetically
    }

    #[test]
    fn test_symbol_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(Symbol::from("AAPL"));
        set.insert(Symbol::from("AAPL")); // Duplicate
        set.insert(Symbol::from("MSFT"));

        assert_eq!(set.len(), 2); // Duplicates are not added
        assert!(set.contains(&Symbol::from("AAPL")));
        assert!(set.contains(&Symbol::from("MSFT")));
    }
}
