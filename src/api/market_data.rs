use std::collections::{BTreeMap, BTreeSet, HashMap};

use rust_decimal::Decimal;
use serde::de::Error as DeError;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;

use crate::api::base::Result;
use crate::TastyTrade;

/// Supported instrument groupings for the market data endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MarketDataParam {
    Cryptocurrency,
    Equity,
    EquityOption,
    Index,
    Future,
    FutureOption,
}

impl MarketDataParam {
    fn as_key(&self) -> &'static str {
        match self {
            Self::Cryptocurrency => "cryptocurrency",
            Self::Equity => "equity",
            Self::EquityOption => "equity-option",
            Self::Index => "index",
            Self::Future => "future",
            Self::FutureOption => "future-option",
        }
    }
}

/// Builder-style container for assembling market data queries by instrument type.
#[derive(Debug, Default, Clone)]
pub struct MarketDataRequest {
    params: BTreeMap<MarketDataParam, BTreeSet<String>>,
}

impl MarketDataRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_symbol(&mut self, param: MarketDataParam, symbol: impl Into<String>) {
        self.params.entry(param).or_default().insert(symbol.into());
    }

    pub fn add_symbols<I, S>(&mut self, param: MarketDataParam, symbols: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let entry = self.params.entry(param).or_default();
        entry.extend(symbols.into_iter().map(Into::into));
    }

    pub fn with_symbol(mut self, param: MarketDataParam, symbol: impl Into<String>) -> Self {
        self.add_symbol(param, symbol);
        self
    }

    pub fn with_symbols<I, S>(mut self, param: MarketDataParam, symbols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.add_symbols(param, symbols);
        self
    }

    pub fn add_equity(&mut self, symbol: impl Into<String>) {
        self.add_symbol(MarketDataParam::Equity, symbol);
    }

    pub fn add_equity_option(&mut self, symbol: impl Into<String>) {
        self.add_symbol(MarketDataParam::EquityOption, symbol);
    }

    pub fn with_equity(self, symbol: impl Into<String>) -> Self {
        self.with_symbol(MarketDataParam::Equity, symbol)
    }

    pub fn with_equity_option(self, symbol: impl Into<String>) -> Self {
        self.with_symbol(MarketDataParam::EquityOption, symbol)
    }

    pub fn is_empty(&self) -> bool {
        self.params.values().all(BTreeSet::is_empty)
    }

    pub fn to_query_pairs(&self) -> Vec<(String, String)> {
        self.params
            .iter()
            .filter_map(|(param, symbols)| {
                if symbols.is_empty() {
                    None
                } else {
                    Some((
                        param.as_key().to_string(),
                        symbols.iter().cloned().collect::<Vec<_>>().join(","),
                    ))
                }
            })
            .collect()
    }
}

/// Normalised representation of an item returned by the market data endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MarketDataItem {
    pub symbol: String,
    pub instrument_type: String,
    pub updated_at: Option<String>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub bid: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub bid_size: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub ask: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub ask_size: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub mid: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub mark: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub last: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub open: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub day_high_price: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub day_low_price: Option<Decimal>,
    #[serde(default)]
    pub close_price_type: Option<String>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub prev_close: Option<Decimal>,
    #[serde(default)]
    pub prev_close_price_type: Option<String>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub year_low_price: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub year_high_price: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub volume: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal", alias = "volatility")]
    pub implied_volatility: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub delta: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub gamma: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub theta: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub vega: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub rho: Option<Decimal>,
    #[serde(default)]
    pub is_trading_halted: Option<bool>,
    #[serde(default)]
    pub halt_start_time: Option<i64>,
    #[serde(default)]
    pub halt_end_time: Option<i64>,
    #[serde(default)]
    pub summary_date: Option<String>,
    #[serde(default)]
    pub prev_close_date: Option<String>,
    #[serde(default)]
    pub close: Option<String>,
    #[serde(default)]
    pub last_mkt: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl TastyTrade {
    pub async fn fetch_market_data(
        &self,
        request: &MarketDataRequest,
    ) -> Result<Vec<MarketDataItem>> {
        let query_pairs = request.to_query_pairs();
        let query_refs: Vec<(&str, &str)> = query_pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let payload: MarketDataPayload = self
            .get_with_query("/market-data/by-type", &query_refs)
            .await?;

        Ok(payload.into_items())
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MarketDataPayload {
    Items { items: Vec<MarketDataItem> },
    Single(MarketDataItem),
}

impl MarketDataPayload {
    fn into_items(self) -> Vec<MarketDataItem> {
        match self {
            MarketDataPayload::Items { items } => items,
            MarketDataPayload::Single(item) => vec![item],
        }
    }
}

fn option_decimal<'de, D>(deserializer: D) -> std::result::Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .to_string()
            .parse::<Decimal>()
            .map(Some)
            .map_err(DeError::custom),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<Decimal>()
                    .map(Some)
                    .map_err(DeError::custom)
            }
        }
        Some(Value::Bool(b)) => Ok(Some(if b { Decimal::ONE } else { Decimal::ZERO })),
        Some(other) => Err(DeError::custom(format!(
            "unexpected value for decimal field: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::str::FromStr;

    fn dec_str(value: &str) -> Decimal {
        Decimal::from_str(value).expect("invalid decimal literal")
    }

    #[test]
    fn request_to_query_pairs_deduplicates() {
        let request = MarketDataRequest::new()
            .with_equity("AAPL")
            .with_equity("MSFT")
            .with_equity_option("AAPL  250119C00150000")
            .with_equity_option("AAPL  250119C00150000");

        let mut pairs = request.to_query_pairs();
        pairs.sort();

        assert_eq!(pairs.len(), 2);
        assert!(pairs.contains(&("equity".to_string(), "AAPL,MSFT".to_string())));
        assert!(pairs.contains(&(
            "equity-option".to_string(),
            "AAPL  250119C00150000".to_string()
        )));
    }

    #[test]
    fn market_data_item_deserializes_core_fields() {
        let json = json!({
            "symbol": "AAPL",
            "instrument-type": "Equity",
            "updated-at": "2025-04-29T21:33:25.535Z",
            "bid": "210.55",
            "bid-size": "2.0",
            "ask": "210.6",
            "ask-size": "1.0",
            "mid": "210.575",
            "mark": "210.55",
            "last": "210.511",
            "open": "208.693",
            "day-high-price": "212.24",
            "day-low-price": "208.37",
            "close-price-type": "Final",
            "prev-close": "210.14",
            "prev-close-price-type": "Final",
            "year-low-price": "169.11",
            "year-high-price": "260.1",
            "volume": "35348839.0",
            "is-trading-halted": false,
            "halt-start-time": -1,
            "halt-end-time": -1,
            "summary-date": "2025-04-29",
            "prev-close-date": "2025-04-28",
            "extra-field": "extra-value"
        });

        let item: MarketDataItem = serde_json::from_value(json).expect("failed to deserialize");
        assert_eq!(item.symbol, "AAPL");
        assert_eq!(item.instrument_type, "Equity");
        assert_eq!(item.bid, Some(dec_str("210.55")));
        assert_eq!(item.ask, Some(dec_str("210.6")));
        assert_eq!(item.bid_size, Some(dec_str("2.0")));
        assert_eq!(item.volume, Some(dec_str("35348839.0")));
        assert_eq!(item.is_trading_halted, Some(false));
        assert_eq!(
            item.extra.get("extra-field"),
            Some(&Value::String("extra-value".to_string()))
        );
    }

    #[test]
    fn option_fields_support_strings() {
        let json = json!({
            "symbol": "AAPL  250119C00150000",
            "instrument-type": "Equity Option",
            "implied-volatility": "0.221",
            "delta": "0.55",
            "gamma": "0.12",
            "theta": "-0.03",
            "vega": "0.15",
            "rho": "0.05"
        });

        let item: MarketDataItem =
            serde_json::from_value(json).expect("failed to deserialize option data");
        assert_eq!(item.implied_volatility, Some(dec_str("0.221")));
        assert_eq!(item.delta, Some(dec_str("0.55")));
        assert_eq!(item.theta, Some(dec_str("-0.03")));
    }

    #[test]
    fn option_fields_support_volatility_alias() {
        let json = json!({
            "symbol": "AAPL  250119C00150000",
            "instrument-type": "Equity Option",
            "volatility": "0.221",  // Note: using "volatility" not "implied-volatility"
            "delta": "0.55",
            "gamma": "0.12",
            "theta": "-0.03",
            "vega": "0.15",
            "rho": "0.05"
        });

        let item: MarketDataItem =
            serde_json::from_value(json).expect("failed to deserialize with volatility alias");
        assert_eq!(item.implied_volatility, Some(dec_str("0.221")));
    }

    #[test]
    fn market_data_payload_accepts_single_object() {
        let json = json!({
            "symbol": "by-type",
            "instrument-type": "Unknown",
            "bid-size": 0,
            "ask-size": 0,
            "is-trading-halted": false
        });

        let payload: MarketDataPayload = serde_json::from_value(json).expect("single payload");
        let items = payload.into_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].symbol, "by-type");
    }

    #[test]
    fn market_data_payload_accepts_items_array() {
        let json = json!({
            "items": [
                {
                    "symbol": "AAPL",
                    "instrument-type": "Equity"
                },
                {
                    "symbol": "TSLA",
                    "instrument-type": "Equity"
                }
            ]
        });

        let payload: MarketDataPayload = serde_json::from_value(json).expect("array payload");
        let items = payload.into_items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].symbol, "AAPL");
        assert_eq!(items[1].symbol, "TSLA");
    }
}
