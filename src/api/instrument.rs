use serde::Deserialize;

use crate::Result;
use crate::TastyTrade;

use super::order::AsSymbol;
use super::order::Symbol;
use super::quote_streaming::DxFeedSymbol;

impl TastyTrade {
    pub async fn get_equity_info(&self, symbol: impl AsSymbol) -> Result<EquityInstrumentInfo> {
        self.get(format!("/instruments/equities/{}", symbol.as_symbol().0))
            .await
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EquityInstrumentInfo {
    pub symbol: Symbol,
    pub streamer_symbol: DxFeedSymbol,
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
}
