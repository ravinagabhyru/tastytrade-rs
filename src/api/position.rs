use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::api::accounts::AccountNumber;

use super::order::{InstrumentType, PriceEffect, Symbol};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum QuantityDirection {
    Long,
    Short,
    Zero,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FullPosition {
    pub account_number: AccountNumber,
    pub symbol: Symbol,
    pub instrument_type: InstrumentType,
    pub underlying_symbol: Symbol,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub quantity: Decimal,
    pub quantity_direction: QuantityDirection,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub close_price: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub average_open_price: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub average_yearly_market_close_price: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub average_daily_market_close_price: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub multiplier: Decimal,
    pub cost_effect: PriceEffect,
    pub is_suppressed: bool,
    pub is_frozen: bool,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub restricted_quantity: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub realized_day_gain: Decimal,
    pub realized_day_gain_effect: String,
    pub realized_day_gain_date: String,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub realized_today: Decimal,
    pub realized_today_effect: String,
    pub realized_today_date: String,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct BriefPosition {
    pub account_number: AccountNumber,
    pub symbol: Symbol,
    pub instrument_type: InstrumentType,
    pub underlying_symbol: Symbol,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub quantity: Decimal,
    pub quantity_direction: QuantityDirection,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub close_price: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub average_open_price: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    pub multiplier: Decimal,
    pub cost_effect: PriceEffect,
    pub is_suppressed: bool,
    pub is_frozen: bool,
    #[serde(with = "rust_decimal::serde::float")]
    pub restricted_quantity: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub realized_day_gain: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub realized_today: Decimal,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_quantity_direction_serde() {
        // Test serialization
        assert_eq!(
            serde_json::to_string(&QuantityDirection::Long).unwrap(),
            "\"Long\""
        );
        assert_eq!(
            serde_json::to_string(&QuantityDirection::Short).unwrap(),
            "\"Short\""
        );
        assert_eq!(
            serde_json::to_string(&QuantityDirection::Zero).unwrap(),
            "\"Zero\""
        );

        // Test deserialization
        let long: QuantityDirection = serde_json::from_str("\"Long\"").unwrap();
        let short: QuantityDirection = serde_json::from_str("\"Short\"").unwrap();
        let zero: QuantityDirection = serde_json::from_str("\"Zero\"").unwrap();

        assert!(matches!(long, QuantityDirection::Long));
        assert!(matches!(short, QuantityDirection::Short));
        assert!(matches!(zero, QuantityDirection::Zero));
    }

    #[test]
    fn test_full_position_deserialization() {
        let json = json!({
            "account-number": "ACC123",
            "symbol": "AAPL",
            "instrument-type": "Equity",
            "underlying-symbol": "AAPL",
            "quantity": "100.00",
            "quantity-direction": "Long",
            "close-price": "150.25",
            "average-open-price": "145.50",
            "average-yearly-market-close-price": "148.75",
            "average-daily-market-close-price": "149.50",
            "multiplier": 1.0,
            "cost-effect": "Debit",
            "is-suppressed": false,
            "is-frozen": false,
            "restricted-quantity": "0.00",
            "realized-day-gain": "475.00",
            "realized-day-gain-effect": "Credit",
            "realized-day-gain-date": "2023-01-01",
            "realized-today": "475.00",
            "realized-today-effect": "Credit",
            "realized-today-date": "2023-01-01",
            "created-at": "2023-01-01T10:00:00Z",
            "updated-at": "2023-01-01T16:00:00Z"
        });

        let position: FullPosition = serde_json::from_value(json).unwrap();
        assert_eq!(position.account_number.0, "ACC123");
        assert_eq!(position.symbol.0, "AAPL");
        assert!(matches!(position.instrument_type, InstrumentType::Equity));
        assert_eq!(position.underlying_symbol.0, "AAPL");
        assert_eq!(position.quantity, Decimal::from_str("100.00").unwrap());
        assert!(matches!(
            position.quantity_direction,
            QuantityDirection::Long
        ));
        assert_eq!(position.close_price, Decimal::from_str("150.25").unwrap());
        assert_eq!(
            position.average_open_price,
            Decimal::from_str("145.50").unwrap()
        );
        assert_eq!(
            position.average_yearly_market_close_price,
            Decimal::from_str("148.75").unwrap()
        );
        assert_eq!(
            position.average_daily_market_close_price,
            Decimal::from_str("149.50").unwrap()
        );
        assert_eq!(position.multiplier, Decimal::from(1));
        assert!(matches!(position.cost_effect, PriceEffect::Debit));
        assert!(!position.is_suppressed);
        assert!(!position.is_frozen);
        assert_eq!(
            position.restricted_quantity,
            Decimal::from_str("0.00").unwrap()
        );
        assert_eq!(
            position.realized_day_gain,
            Decimal::from_str("475.00").unwrap()
        );
        assert_eq!(position.realized_day_gain_effect, "Credit");
        assert_eq!(position.realized_day_gain_date, "2023-01-01");
        assert_eq!(
            position.realized_today,
            Decimal::from_str("475.00").unwrap()
        );
        assert_eq!(position.realized_today_effect, "Credit");
        assert_eq!(position.realized_today_date, "2023-01-01");
        assert_eq!(position.created_at, "2023-01-01T10:00:00Z");
        assert_eq!(position.updated_at, "2023-01-01T16:00:00Z");
    }

    #[test]
    fn test_brief_position_deserialization() {
        let json = json!({
            "account-number": "ACC456",
            "symbol": "SPY  240115C00450000",
            "instrument-type": "Equity Option",
            "underlying-symbol": "SPY",
            "quantity": "5.00",
            "quantity-direction": "Short",
            "close-price": "2.50",
            "average-open-price": "3.25",
            "multiplier": 100.0,
            "cost-effect": "Credit",
            "is-suppressed": true,
            "is-frozen": false,
            "restricted-quantity": 1.5,
            "realized-day-gain": "-375.00",
            "realized-today": "-375.00",
            "created-at": "2023-01-01T09:30:00Z",
            "updated-at": "2023-01-01T15:30:00Z"
        });

        let position: BriefPosition = serde_json::from_value(json).unwrap();
        assert_eq!(position.account_number.0, "ACC456");
        assert_eq!(position.symbol.0, "SPY  240115C00450000");
        assert!(matches!(
            position.instrument_type,
            InstrumentType::EquityOption
        ));
        assert_eq!(position.underlying_symbol.0, "SPY");
        assert_eq!(position.quantity, Decimal::from_str("5.00").unwrap());
        assert!(matches!(
            position.quantity_direction,
            QuantityDirection::Short
        ));
        assert_eq!(position.close_price, Decimal::from_str("2.50").unwrap());
        assert_eq!(
            position.average_open_price,
            Decimal::from_str("3.25").unwrap()
        );
        assert_eq!(position.multiplier, Decimal::from(100));
        assert!(matches!(position.cost_effect, PriceEffect::Credit));
        assert!(position.is_suppressed);
        assert!(!position.is_frozen);
        // Note: BriefPosition has restricted_quantity as float, not arbitrary_precision
        assert_eq!(
            position.restricted_quantity,
            Decimal::try_from(1.5).unwrap()
        );
        assert_eq!(
            position.realized_day_gain,
            Decimal::from_str("-375.00").unwrap()
        );
        assert_eq!(
            position.realized_today,
            Decimal::from_str("-375.00").unwrap()
        );
        assert_eq!(position.created_at, "2023-01-01T09:30:00Z");
        assert_eq!(position.updated_at, "2023-01-01T15:30:00Z");
    }

    #[test]
    fn test_full_position_arbitrary_precision_fields() {
        let json = json!({
            "account-number": "PREC789",
            "symbol": "TEST",
            "instrument-type": "Equity",
            "underlying-symbol": "TEST",
            "quantity": "123.123456789",
            "quantity-direction": "Long",
            "close-price": "999.999999999",
            "average-open-price": "888.888888888",
            "average-yearly-market-close-price": "777.777777777",
            "average-daily-market-close-price": "666.666666666",
            "multiplier": 1.5,  // This uses float precision
            "cost-effect": "Debit",
            "is-suppressed": false,
            "is-frozen": false,
            "restricted-quantity": "0.000000001",
            "realized-day-gain": "12345.123456789",
            "realized-day-gain-effect": "Credit",
            "realized-day-gain-date": "2023-01-01",
            "realized-today": "54321.987654321",
            "realized-today-effect": "Debit",
            "realized-today-date": "2023-01-01",
            "created-at": "2023-01-01T10:00:00Z",
            "updated-at": "2023-01-01T16:00:00Z"
        });

        let position: FullPosition = serde_json::from_value(json).unwrap();

        // Test high precision preservation for arbitrary_precision fields
        assert_eq!(
            position.quantity,
            Decimal::from_str("123.123456789").unwrap()
        );
        assert_eq!(
            position.close_price,
            Decimal::from_str("999.999999999").unwrap()
        );
        assert_eq!(
            position.average_open_price,
            Decimal::from_str("888.888888888").unwrap()
        );
        assert_eq!(
            position.average_yearly_market_close_price,
            Decimal::from_str("777.777777777").unwrap()
        );
        assert_eq!(
            position.average_daily_market_close_price,
            Decimal::from_str("666.666666666").unwrap()
        );
        assert_eq!(
            position.restricted_quantity,
            Decimal::from_str("0.000000001").unwrap()
        );
        assert_eq!(
            position.realized_day_gain,
            Decimal::from_str("12345.123456789").unwrap()
        );
        assert_eq!(
            position.realized_today,
            Decimal::from_str("54321.987654321").unwrap()
        );

        // multiplier uses float precision
        assert_eq!(position.multiplier, Decimal::try_from(1.5).unwrap());
    }

    #[test]
    fn test_brief_position_float_vs_arbitrary_precision() {
        // Test that BriefPosition has different precision handling than FullPosition
        let json = json!({
            "account-number": "FLOAT123",
            "symbol": "TEST",
            "instrument-type": "Future",
            "underlying-symbol": "TEST",
            "quantity": "100.123456789",  // arbitrary_precision
            "quantity-direction": "Zero",
            "close-price": "50.987654321",  // arbitrary_precision
            "average-open-price": "51.111111111",  // arbitrary_precision
            "multiplier": 2.75,  // float
            "cost-effect": "None",
            "is-suppressed": false,
            "is-frozen": true,
            "restricted-quantity": 3.14159,  // float (different from FullPosition)
            "realized-day-gain": "1234.56789",  // arbitrary_precision
            "realized-today": "9876.54321",  // arbitrary_precision
            "created-at": "2023-01-01T12:00:00Z",
            "updated-at": "2023-01-01T18:00:00Z"
        });

        let position: BriefPosition = serde_json::from_value(json).unwrap();

        // Test arbitrary precision fields
        assert_eq!(
            position.quantity,
            Decimal::from_str("100.123456789").unwrap()
        );
        assert_eq!(
            position.close_price,
            Decimal::from_str("50.987654321").unwrap()
        );
        assert_eq!(
            position.average_open_price,
            Decimal::from_str("51.111111111").unwrap()
        );
        assert_eq!(
            position.realized_day_gain,
            Decimal::from_str("1234.56789").unwrap()
        );
        assert_eq!(
            position.realized_today,
            Decimal::from_str("9876.54321").unwrap()
        );

        // Test float precision fields
        assert_eq!(position.multiplier, Decimal::try_from(2.75).unwrap());
        // restricted_quantity is float in BriefPosition (different from FullPosition)
        assert_eq!(
            position.restricted_quantity,
            Decimal::try_from(3.14159).unwrap()
        );

        assert!(matches!(
            position.quantity_direction,
            QuantityDirection::Zero
        ));
        assert!(matches!(position.cost_effect, PriceEffect::None));
        assert!(!position.is_suppressed);
        assert!(position.is_frozen);
    }

    #[test]
    fn test_position_copy_and_clone() {
        let direction = QuantityDirection::Long;
        let direction_copy = direction; // Test Copy
        let direction_clone = direction.clone(); // Test Clone

        assert!(matches!(direction, QuantityDirection::Long));
        assert!(matches!(direction_copy, QuantityDirection::Long));
        assert!(matches!(direction_clone, QuantityDirection::Long));
    }
}
