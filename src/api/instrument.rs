use serde::Deserialize;

use crate::Result;
use crate::TastyTrade;

use super::order::AsSymbol;
use super::order::Symbol;
use super::quote_streaming::DxFeedSymbol;

impl TastyTrade {
    /// Get equity instrument info for a single symbol.
    ///
    /// # Example
    /// ```ignore
    /// let info = client.get_equity_info("AAPL").await?;
    /// println!("Company: {}", info.short_description.unwrap_or_default());
    /// ```
    pub async fn get_equity_info(&self, symbol: impl AsSymbol) -> Result<EquityInstrumentInfo> {
        self.get(format!("/instruments/equities/{}", symbol.as_symbol().0))
            .await
    }

    /// Get equity instrument info for multiple symbols in a single request.
    ///
    /// # Example
    /// ```ignore
    /// let infos = client.get_equities_info(&["AAPL", "SPY", "MSFT"]).await?;
    /// for info in infos {
    ///     println!("{}: {}", info.symbol.0, info.short_description.unwrap_or_default());
    /// }
    /// ```
    pub async fn get_equities_info(&self, symbols: &[&str]) -> Result<Vec<EquityInstrumentInfo>> {
        if symbols.is_empty() {
            return Ok(vec![]);
        }

        // Build query string: symbol[]=AAPL&symbol[]=SPY
        let query: String = symbols
            .iter()
            .map(|s| format!("symbol[]={}", s))
            .collect::<Vec<_>>()
            .join("&");

        self.get(format!("/instruments/equities?{}", query)).await
    }
}

/// Tick size threshold for equity trading.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TickSize {
    /// The tick size value (e.g., "0.01")
    pub value: String,
    /// Price threshold below which this tick size applies (optional)
    pub threshold: Option<String>,
}

/// Complete equity instrument information from TastyTrade API.
///
/// Contains all available metadata about an equity instrument including
/// company name, trading characteristics, and market information.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EquityInstrumentInfo {
    /// Unique instrument ID
    pub id: Option<u64>,

    /// Ticker symbol (e.g., "AAPL")
    pub symbol: Symbol,

    /// Instrument type (always "Equity" for this struct)
    pub instrument_type: Option<String>,

    /// CUSIP identifier
    pub cusip: Option<String>,

    /// Short company/security name (e.g., "APPLE INC")
    pub short_description: Option<String>,

    /// Full description of the equity
    pub description: Option<String>,

    /// Whether this is an index (e.g., SPX)
    pub is_index: Option<bool>,

    /// Listed market/exchange (e.g., "XNAS" for NASDAQ)
    pub listed_market: Option<String>,

    /// Lendability status: "Easy To Borrow", "Locate Required", "Preborrow"
    pub lendability: Option<String>,

    /// Borrow rate for short selling
    pub borrow_rate: Option<String>,

    /// Market time instrument collection (e.g., "Equity", "Equity Index")
    pub market_time_instrument_collection: Option<String>,

    /// Whether equity is closing-only (no new positions)
    pub is_closing_only: Option<bool>,

    /// Whether options are closing-only
    pub is_options_closing_only: Option<bool>,

    /// Whether the instrument is currently active/tradeable
    pub active: Option<bool>,

    /// Whether fractional share trading is supported
    pub is_fractional_quantity_eligible: Option<bool>,

    /// Whether the instrument is illiquid
    pub is_illiquid: Option<bool>,

    /// Whether this is an ETF
    pub is_etf: Option<bool>,

    /// Symbol used for DxLink streaming subscriptions
    pub streamer_symbol: DxFeedSymbol,

    /// Tick sizes for equity trading
    pub tick_sizes: Option<Vec<TickSize>>,

    /// Tick sizes for options on this equity
    pub option_tick_sizes: Option<Vec<TickSize>>,
}

impl EquityInstrumentInfo {
    /// Returns the company/security name, preferring short_description.
    ///
    /// Falls back to description, then symbol if neither is available.
    pub fn name(&self) -> &str {
        self.short_description
            .as_deref()
            .or(self.description.as_deref())
            .unwrap_or(&self.symbol.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_equity_instrument_info_deserialization() {
        let json = json!({
            "symbol": "AAPL",
            "streamer-symbol": "AAPL"
        });

        let info: EquityInstrumentInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.symbol.0, "AAPL");
        assert_eq!(info.streamer_symbol.0, "AAPL");
    }

    #[test]
    fn test_equity_instrument_info_kebab_case_fields() {
        // Test that kebab-case field names are correctly mapped
        let json = json!({
            "symbol": "SPY",
            "streamer-symbol": "SPY_123"
        });

        let info: EquityInstrumentInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.symbol.0, "SPY");
        assert_eq!(info.streamer_symbol.0, "SPY_123");
    }

    #[test]
    fn test_equity_instrument_info_different_symbols() {
        // Test case where the symbol and streamer_symbol differ
        let json = json!({
            "symbol": "GOOGL",
            "streamer-symbol": ".GOOGL.NASDAQ"
        });

        let info: EquityInstrumentInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.symbol.0, "GOOGL");
        assert_eq!(info.streamer_symbol.0, ".GOOGL.NASDAQ");
    }

    #[test]
    fn test_equity_instrument_info_full_deserialization() {
        // Test full API response with all fields
        let json = json!({
            "id": 726,
            "symbol": "AAPL",
            "instrument-type": "Equity",
            "cusip": "037833100",
            "short-description": "APPLE INC",
            "is-index": false,
            "listed-market": "XNAS",
            "description": "APPLE INC",
            "lendability": "Easy To Borrow",
            "borrow-rate": "0.0",
            "market-time-instrument-collection": "Equity",
            "is-closing-only": false,
            "is-options-closing-only": false,
            "active": true,
            "is-fractional-quantity-eligible": true,
            "is-illiquid": false,
            "is-etf": false,
            "streamer-symbol": "AAPL",
            "tick-sizes": [
                { "value": "0.0001", "threshold": "1.0" },
                { "value": "0.01" }
            ],
            "option-tick-sizes": [
                { "value": "0.01", "threshold": "3.0" },
                { "value": "0.05" }
            ]
        });

        let info: EquityInstrumentInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.id, Some(726));
        assert_eq!(info.symbol.0, "AAPL");
        assert_eq!(info.instrument_type, Some("Equity".to_string()));
        assert_eq!(info.cusip, Some("037833100".to_string()));
        assert_eq!(info.short_description, Some("APPLE INC".to_string()));
        assert_eq!(info.description, Some("APPLE INC".to_string()));
        assert_eq!(info.is_index, Some(false));
        assert_eq!(info.listed_market, Some("XNAS".to_string()));
        assert_eq!(info.lendability, Some("Easy To Borrow".to_string()));
        assert_eq!(info.borrow_rate, Some("0.0".to_string()));
        assert_eq!(info.is_closing_only, Some(false));
        assert_eq!(info.is_options_closing_only, Some(false));
        assert_eq!(info.active, Some(true));
        assert_eq!(info.is_fractional_quantity_eligible, Some(true));
        assert_eq!(info.is_illiquid, Some(false));
        assert_eq!(info.is_etf, Some(false));
        assert_eq!(info.streamer_symbol.0, "AAPL");
        assert!(info.tick_sizes.is_some());
        assert_eq!(info.tick_sizes.as_ref().unwrap().len(), 2);
        assert!(info.option_tick_sizes.is_some());
        assert_eq!(info.option_tick_sizes.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_equity_instrument_info_index() {
        // Test index instrument (like SPX)
        let json = json!({
            "id": 17702,
            "symbol": "SPX",
            "instrument-type": "Equity",
            "cusip": "648815108",
            "short-description": "S & P 500 INDEX",
            "is-index": true,
            "listed-market": "OTC",
            "description": "S & P 500 INDEX",
            "lendability": "Locate Required",
            "borrow-rate": "0.0",
            "market-time-instrument-collection": "Equity Index",
            "is-closing-only": false,
            "is-options-closing-only": false,
            "active": false,
            "is-fractional-quantity-eligible": false,
            "is-illiquid": false,
            "is-etf": false,
            "streamer-symbol": "SPX"
        });

        let info: EquityInstrumentInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.is_index, Some(true));
        assert_eq!(info.short_description, Some("S & P 500 INDEX".to_string()));
        assert_eq!(info.market_time_instrument_collection, Some("Equity Index".to_string()));
    }

    #[test]
    fn test_name_helper() {
        // Test the name() helper method
        let json = json!({
            "symbol": "AAPL",
            "short-description": "APPLE INC",
            "description": "Apple Inc. - Full Description",
            "streamer-symbol": "AAPL"
        });

        let info: EquityInstrumentInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.name(), "APPLE INC"); // Prefers short_description

        // Test fallback to description
        let json2 = json!({
            "symbol": "TEST",
            "description": "Test Company",
            "streamer-symbol": "TEST"
        });

        let info2: EquityInstrumentInfo = serde_json::from_value(json2).unwrap();
        assert_eq!(info2.name(), "Test Company");

        // Test fallback to symbol
        let json3 = json!({
            "symbol": "XYZ",
            "streamer-symbol": "XYZ"
        });

        let info3: EquityInstrumentInfo = serde_json::from_value(json3).unwrap();
        assert_eq!(info3.name(), "XYZ");
    }

    #[test]
    fn test_tick_size_deserialization() {
        let json = json!({
            "value": "0.01",
            "threshold": "3.0"
        });

        let tick: TickSize = serde_json::from_value(json).unwrap();
        assert_eq!(tick.value, "0.01");
        assert_eq!(tick.threshold, Some("3.0".to_string()));

        // Test without threshold
        let json2 = json!({
            "value": "0.05"
        });

        let tick2: TickSize = serde_json::from_value(json2).unwrap();
        assert_eq!(tick2.value, "0.05");
        assert_eq!(tick2.threshold, None);
    }
}
