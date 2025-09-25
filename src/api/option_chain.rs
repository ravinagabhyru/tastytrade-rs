use std::collections::HashMap;

use crate::api::base::Result;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;

use crate::TastyTrade;

use super::{
    base::Items,
    order::{AsSymbol, Symbol},
    quote_streaming::DxFeedSymbol,
};

impl TastyTrade {
    pub async fn nested_option_chain_for(
        &self,
        symbol: impl Into<Symbol>,
    ) -> Result<NestedOptionChain> {
        let symbol = symbol.into();
        let mut resp: Items<NestedOptionChain> = self
            .get(format!("/option-chains/{}/nested", symbol.0))
            .await?;

        if resp.items.is_empty() {
            return Err(crate::api::base::ApiError {
                message: format!("No option chain found for symbol: {}", symbol.0),
                code: None,
                errors: None,
            }
            .into());
        }

        Ok(resp.items.remove(0))
    }

    pub async fn option_chain_for(&self, symbol: impl Into<Symbol>) -> Result<Vec<OptionChain>> {
        let resp: Items<OptionChain> = self
            .get(format!("/option-chains/{}", symbol.into().0))
            .await?;
        Ok(resp.items)
    }

    pub async fn get_option_info(&self, symbol: impl AsSymbol) -> Result<OptionInfo> {
        self.get(format!(
            "/instruments/equity-options/{}",
            symbol.as_symbol().0
        ))
        .await
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OptionInfo {
    pub streamer_symbol: DxFeedSymbol,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NestedOptionChain {
    pub underlying_symbol: Symbol,
    pub root_symbol: Symbol,
    pub option_chain_type: String,
    pub shares_per_contract: u64,
    pub expirations: Vec<Expiration>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Expiration {
    pub expiration_type: String,
    pub expiration_date: String,
    pub days_to_expiration: u64,
    pub settlement_type: String,
    pub strikes: Vec<Strike>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Strike {
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub strike_price: Decimal,
    pub call: Symbol,
    pub put: Symbol,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OptionChain {
    pub underlying_symbol: Symbol,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub strike_price: Decimal,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_nested_option_chain_deserialization() {
        let json = json!({
            "underlying-symbol": "AAPL",
            "root-symbol": "AAPL",
            "option-chain-type": "Standard",
            "shares-per-contract": 100,
            "expirations": [
                {
                    "expiration-type": "Regular",
                    "expiration-date": "2024-01-19",
                    "days-to-expiration": 30,
                    "settlement-type": "PM",
                    "strikes": [
                        {
                            "strike-price": "150.00",
                            "call": "AAPL  240119C00150000",
                            "put": "AAPL  240119P00150000"
                        },
                        {
                            "strike-price": "155.00",
                            "call": "AAPL  240119C00155000",
                            "put": "AAPL  240119P00155000"
                        }
                    ]
                },
                {
                    "expiration-type": "Weekly",
                    "expiration-date": "2024-01-12",
                    "days-to-expiration": 23,
                    "settlement-type": "PM",
                    "strikes": [
                        {
                            "strike-price": "160.00",
                            "call": "AAPL  240112C00160000",
                            "put": "AAPL  240112P00160000"
                        }
                    ]
                }
            ]
        });

        let chain: NestedOptionChain = serde_json::from_value(json).unwrap();
        assert_eq!(chain.underlying_symbol.0, "AAPL");
        assert_eq!(chain.root_symbol.0, "AAPL");
        assert_eq!(chain.option_chain_type, "Standard");
        assert_eq!(chain.shares_per_contract, 100);
        assert_eq!(chain.expirations.len(), 2);

        // Test first expiration
        let exp1 = &chain.expirations[0];
        assert_eq!(exp1.expiration_type, "Regular");
        assert_eq!(exp1.expiration_date, "2024-01-19");
        assert_eq!(exp1.days_to_expiration, 30);
        assert_eq!(exp1.settlement_type, "PM");
        assert_eq!(exp1.strikes.len(), 2);

        // Test strike prices with high precision
        assert_eq!(
            exp1.strikes[0].strike_price,
            Decimal::from_str("150.00").unwrap()
        );
        assert_eq!(exp1.strikes[0].call.0, "AAPL  240119C00150000");
        assert_eq!(exp1.strikes[0].put.0, "AAPL  240119P00150000");

        assert_eq!(
            exp1.strikes[1].strike_price,
            Decimal::from_str("155.00").unwrap()
        );
        assert_eq!(exp1.strikes[1].call.0, "AAPL  240119C00155000");
        assert_eq!(exp1.strikes[1].put.0, "AAPL  240119P00155000");

        // Test second expiration
        let exp2 = &chain.expirations[1];
        assert_eq!(exp2.expiration_type, "Weekly");
        assert_eq!(exp2.expiration_date, "2024-01-12");
        assert_eq!(exp2.days_to_expiration, 23);
        assert_eq!(exp2.settlement_type, "PM");
        assert_eq!(exp2.strikes.len(), 1);
        assert_eq!(
            exp2.strikes[0].strike_price,
            Decimal::from_str("160.00").unwrap()
        );
    }

    #[test]
    fn test_option_chain_deserialization_with_flatten() {
        let json = json!({
            "underlying-symbol": "SPY",
            "strike-price": "450.00",
            // Extra fields that should be captured by #[serde(flatten)]
            "volume": 1500,
            "open-interest": 2500,
            "implied-volatility": 0.25,
            "delta": 0.55,
            "gamma": 0.02,
            "theta": -0.15,
            "vega": 1.25,
            "custom-field": "test_value"
        });

        let chain: OptionChain = serde_json::from_value(json).unwrap();
        assert_eq!(chain.underlying_symbol.0, "SPY");
        assert_eq!(chain.strike_price, Decimal::from_str("450.00").unwrap());

        // Verify that extra fields are captured in the HashMap
        assert!(chain.extra.contains_key("volume"));
        assert!(chain.extra.contains_key("open-interest"));
        assert!(chain.extra.contains_key("implied-volatility"));
        assert!(chain.extra.contains_key("delta"));
        assert!(chain.extra.contains_key("gamma"));
        assert!(chain.extra.contains_key("theta"));
        assert!(chain.extra.contains_key("vega"));
        assert!(chain.extra.contains_key("custom-field"));

        // Test that the values are correctly preserved
        assert_eq!(chain.extra["volume"], json!(1500));
        assert_eq!(chain.extra["open-interest"], json!(2500));
        assert_eq!(chain.extra["implied-volatility"], json!(0.25));
        assert_eq!(chain.extra["custom-field"], json!("test_value"));
    }

    #[test]
    fn test_option_info_deserialization() {
        let json = json!({
            "streamer-symbol": "AAPL  240119C00150000"
        });

        let option_info: OptionInfo = serde_json::from_value(json).unwrap();
        assert_eq!(option_info.streamer_symbol.0, "AAPL  240119C00150000");
    }

    #[test]
    fn test_strike_high_precision_decimal() {
        let json = json!({
            "strike-price": "123.456789012345",
            "call": "TEST  240119C00123456",
            "put": "TEST  240119P00123456"
        });

        let strike: Strike = serde_json::from_value(json).unwrap();
        // Test that arbitrary precision is preserved
        assert_eq!(
            strike.strike_price,
            Decimal::from_str("123.456789012345").unwrap()
        );
        assert_eq!(strike.call.0, "TEST  240119C00123456");
        assert_eq!(strike.put.0, "TEST  240119P00123456");
    }

    #[test]
    fn test_option_chain_high_precision_strike_price() {
        let json = json!({
            "underlying-symbol": "TEST",
            "strike-price": "999.123456789",
            "extra-field": "should-be-in-extra-map"
        });

        let chain: OptionChain = serde_json::from_value(json).unwrap();
        assert_eq!(chain.underlying_symbol.0, "TEST");
        // Test that arbitrary precision is preserved for strike_price
        assert_eq!(
            chain.strike_price,
            Decimal::from_str("999.123456789").unwrap()
        );
        assert_eq!(chain.extra["extra-field"], json!("should-be-in-extra-map"));
    }

    #[test]
    fn test_nested_option_chain_complex_structure() {
        // Test with a more complex nested structure
        let json = json!({
            "underlying-symbol": "SPX",
            "root-symbol": "SPXW",
            "option-chain-type": "Index",
            "shares-per-contract": 1,
            "expirations": [
                {
                    "expiration-type": "AM",
                    "expiration-date": "2024-02-01",
                    "days-to-expiration": 45,
                    "settlement-type": "AM",
                    "strikes": [
                        {
                            "strike-price": "4500.00",
                            "call": "SPXW 240201C04500000",
                            "put": "SPXW 240201P04500000"
                        },
                        {
                            "strike-price": "4525.00",
                            "call": "SPXW 240201C04525000",
                            "put": "SPXW 240201P04525000"
                        },
                        {
                            "strike-price": "4550.50",
                            "call": "SPXW 240201C04550500",
                            "put": "SPXW 240201P04550500"
                        }
                    ]
                }
            ]
        });

        let chain: NestedOptionChain = serde_json::from_value(json).unwrap();
        assert_eq!(chain.underlying_symbol.0, "SPX");
        assert_eq!(chain.root_symbol.0, "SPXW");
        assert_eq!(chain.option_chain_type, "Index");
        assert_eq!(chain.shares_per_contract, 1); // Index options typically have multiplier of 1

        let exp = &chain.expirations[0];
        assert_eq!(exp.expiration_type, "AM");
        assert_eq!(exp.settlement_type, "AM");
        assert_eq!(exp.strikes.len(), 3);

        // Test decimal precision for different strike prices
        assert_eq!(
            exp.strikes[0].strike_price,
            Decimal::from_str("4500.00").unwrap()
        );
        assert_eq!(
            exp.strikes[1].strike_price,
            Decimal::from_str("4525.00").unwrap()
        );
        assert_eq!(
            exp.strikes[2].strike_price,
            Decimal::from_str("4550.50").unwrap()
        );

        // Test symbol formatting
        assert_eq!(exp.strikes[2].call.0, "SPXW 240201C04550500");
        assert_eq!(exp.strikes[2].put.0, "SPXW 240201P04550500");
    }

    #[test]
    fn test_option_chain_empty_extra_fields() {
        // Test that OptionChain works even with minimal required fields
        let json = json!({
            "underlying-symbol": "TSLA",
            "strike-price": "200.00"
        });

        let chain: OptionChain = serde_json::from_value(json).unwrap();
        assert_eq!(chain.underlying_symbol.0, "TSLA");
        assert_eq!(chain.strike_price, Decimal::from_str("200.00").unwrap());
        assert!(chain.extra.is_empty()); // No extra fields should result in empty HashMap
    }

    #[test]
    fn test_dxfeed_symbol_serde() {
        let symbol = DxFeedSymbol("TEST123".to_string());

        // Test serialization
        let serialized = serde_json::to_string(&symbol).unwrap();
        assert_eq!(serialized, "\"TEST123\"");

        // Test deserialization
        let deserialized: DxFeedSymbol = serde_json::from_str("\"TEST123\"").unwrap();
        assert_eq!(deserialized.0, "TEST123");
    }
}
