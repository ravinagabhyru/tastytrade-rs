use rust_decimal::Decimal;
use serde::de::{Error as DeError, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::accounts::AccountNumber;
use super::order::{Action, InstrumentType, OrderId, PriceEffect, Symbol};

// Helper for deserializing Option<Decimal> with arbitrary precision
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
            "Invalid type for decimal: {:?}",
            other
        ))),
    }
}

// ============================================================================
// Transaction ID
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
pub struct TransactionId(pub u64);

// ============================================================================
// Transaction Type Enums
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TransactionType {
    #[serde(rename = "Administrative Transfer")]
    AdministrativeTransfer,
    #[serde(rename = "Money Movement")]
    MoneyMovement,
    #[serde(rename = "Receive Deliver")]
    ReceiveDeliver,
    Trade,
    #[serde(other)]
    Unknown,
}

impl TransactionType {
    pub fn as_api_str(&self) -> &str {
        match self {
            TransactionType::AdministrativeTransfer => "Administrative Transfer",
            TransactionType::MoneyMovement => "Money Movement",
            TransactionType::ReceiveDeliver => "Receive Deliver",
            TransactionType::Trade => "Trade",
            TransactionType::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TransactionSubType {
    ACAT,
    Assignment,
    #[serde(rename = "Balance Adjustment")]
    BalanceAdjustment,
    #[serde(rename = "Cash Merger")]
    CashMerger,
    #[serde(rename = "Cash Settled Assignment")]
    CashSettledAssignment,
    #[serde(rename = "Cash Settled Exercise")]
    CashSettledExercise,
    #[serde(rename = "Credit Interest")]
    CreditInterest,
    #[serde(rename = "Debit Interest")]
    DebitInterest,
    Deposit,
    Dividend,
    Exercise,
    Expiration,
    Fee,
    #[serde(rename = "Forward Split")]
    ForwardSplit,
    #[serde(rename = "Fully Paid Stock Lending Income")]
    FullyPaidStockLendingIncome,
    #[serde(rename = "Futures Settlement")]
    FuturesSettlement,
    #[serde(rename = "Mark to Market")]
    MarkToMarket,
    Maturity,
    #[serde(rename = "Reverse Split")]
    ReverseSplit,
    #[serde(rename = "Reverse Split Removal")]
    ReverseSplitRemoval,
    #[serde(rename = "Special Dividend")]
    SpecialDividend,
    #[serde(rename = "Stock Merger")]
    StockMerger,
    #[serde(rename = "Stock Merger Removal")]
    StockMergerRemoval,
    #[serde(rename = "Symbol Change")]
    SymbolChange,
    Transfer,
    Withdrawal,
    #[serde(other)]
    Unknown,
}

impl TransactionSubType {
    pub fn as_api_str(&self) -> &str {
        match self {
            TransactionSubType::ACAT => "ACAT",
            TransactionSubType::Assignment => "Assignment",
            TransactionSubType::BalanceAdjustment => "Balance Adjustment",
            TransactionSubType::CashMerger => "Cash Merger",
            TransactionSubType::CashSettledAssignment => "Cash Settled Assignment",
            TransactionSubType::CashSettledExercise => "Cash Settled Exercise",
            TransactionSubType::CreditInterest => "Credit Interest",
            TransactionSubType::DebitInterest => "Debit Interest",
            TransactionSubType::Deposit => "Deposit",
            TransactionSubType::Dividend => "Dividend",
            TransactionSubType::Exercise => "Exercise",
            TransactionSubType::Expiration => "Expiration",
            TransactionSubType::Fee => "Fee",
            TransactionSubType::ForwardSplit => "Forward Split",
            TransactionSubType::FullyPaidStockLendingIncome => "Fully Paid Stock Lending Income",
            TransactionSubType::FuturesSettlement => "Futures Settlement",
            TransactionSubType::MarkToMarket => "Mark to Market",
            TransactionSubType::Maturity => "Maturity",
            TransactionSubType::ReverseSplit => "Reverse Split",
            TransactionSubType::ReverseSplitRemoval => "Reverse Split Removal",
            TransactionSubType::SpecialDividend => "Special Dividend",
            TransactionSubType::StockMerger => "Stock Merger",
            TransactionSubType::StockMergerRemoval => "Stock Merger Removal",
            TransactionSubType::SymbolChange => "Symbol Change",
            TransactionSubType::Transfer => "Transfer",
            TransactionSubType::Withdrawal => "Withdrawal",
            TransactionSubType::Unknown => "Unknown",
        }
    }
}

// ============================================================================
// Transaction Lot
// ============================================================================

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TransactionLot {
    pub lot_id: Option<u64>,
    pub symbol: Option<Symbol>,
    pub instrument_type: Option<InstrumentType>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub quantity: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub price: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub value: Option<Decimal>,
}

// ============================================================================
// Transaction
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Transaction {
    pub id: TransactionId,
    pub account_number: AccountNumber,
    pub symbol: Option<Symbol>,
    pub instrument_type: Option<InstrumentType>,
    pub underlying_symbol: Option<Symbol>,
    pub transaction_type: TransactionType,
    pub transaction_sub_type: TransactionSubType,
    pub description: String,
    pub action: Option<Action>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub quantity: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub price: Option<Decimal>,
    pub executed_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub transaction_date: chrono::NaiveDate,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub value: Decimal,
    pub value_effect: PriceEffect,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub net_value: Decimal,
    pub net_value_effect: PriceEffect,
    #[serde(default, deserialize_with = "option_decimal")]
    pub cash_balance: Option<Decimal>,
    pub cash_balance_effect: Option<PriceEffect>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub commission: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub clearing_fees: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub regulatory_fees: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub occ_fees: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub proprietary_fees: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub other_fees: Option<Decimal>,
    #[serde(default, deserialize_with = "option_decimal")]
    pub accrued_interest: Option<Decimal>,
    pub settlement_date: Option<chrono::NaiveDate>,
    pub futures_symbol: Option<String>,
    pub partition_key: Option<String>,
    pub instrument_id: Option<u64>,
    pub order_id: Option<OrderId>,
    pub execution_id: Option<String>,
    pub trade_transaction_type: Option<String>,
    #[serde(default)]
    pub lots: Vec<TransactionLot>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_estimated_fee: bool,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

// ============================================================================
// Total Fees
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TotalFees {
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub total_fees: Decimal,
    pub total_fees_effect: PriceEffect,
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Clone, Copy)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    pub fn as_api_str(&self) -> &str {
        match self {
            SortOrder::Asc => "Asc",
            SortOrder::Desc => "Desc",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TransactionQueryError {
    #[error("transaction query cannot use both type and types[] filters simultaneously")]
    ConflictingTypeFilters,
    #[error("transaction query cannot use both date filters (start-date/end-date) and timestamp filters (start-at/end-at) simultaneously")]
    ConflictingDateFilters,
}

#[derive(Default, Clone)]
pub struct TransactionQueryParams {
    pub sort: Option<SortOrder>,
    pub transaction_type: Option<TransactionType>,
    pub transaction_types: Vec<TransactionType>,
    pub sub_types: Vec<TransactionSubType>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub start_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub end_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub instrument_type: Option<InstrumentType>,
    pub symbol: Option<Symbol>,
    pub underlying_symbol: Option<Symbol>,
    pub action: Option<Action>,
    pub partition_key: Option<String>,
    pub futures_symbol: Option<String>,
    pub page_offset: Option<usize>,
    pub per_page: Option<usize>,
}

impl TransactionQueryParams {
    pub fn validate(&self) -> std::result::Result<(), TransactionQueryError> {
        // Check for conflicting type filters
        if self.transaction_type.is_some() && !self.transaction_types.is_empty() {
            return Err(TransactionQueryError::ConflictingTypeFilters);
        }

        // Check for conflicting date filters
        let has_date_filters = self.start_date.is_some() || self.end_date.is_some();
        let has_timestamp_filters = self.start_at.is_some() || self.end_at.is_some();
        if has_date_filters && has_timestamp_filters {
            return Err(TransactionQueryError::ConflictingDateFilters);
        }

        Ok(())
    }

    pub fn into_query(self) -> Vec<(String, String)> {
        let mut params = Vec::new();

        if let Some(sort) = self.sort {
            params.push(("sort".to_string(), sort.as_api_str().to_string()));
        }
        if let Some(t) = self.transaction_type {
            params.push(("type".to_string(), t.as_api_str().to_string()));
        }
        for t in self.transaction_types {
            params.push(("types[]".to_string(), t.as_api_str().to_string()));
        }
        for sub in self.sub_types {
            params.push(("sub-type[]".to_string(), sub.as_api_str().to_string()));
        }
        if let Some(date) = self.start_date {
            params.push(("start-date".to_string(), date.to_string()));
        }
        if let Some(date) = self.end_date {
            params.push(("end-date".to_string(), date.to_string()));
        }
        if let Some(ts) = self.start_at {
            params.push(("start-at".to_string(), ts.to_rfc3339()));
        }
        if let Some(ts) = self.end_at {
            params.push(("end-at".to_string(), ts.to_rfc3339()));
        }
        if let Some(instr) = self.instrument_type {
            params.push((
                "instrument-type".to_string(),
                instr.as_api_str().to_string(),
            ));
        }
        if let Some(symbol) = self.symbol {
            params.push(("symbol".to_string(), symbol.0));
        }
        if let Some(symbol) = self.underlying_symbol {
            params.push(("underlying-symbol".to_string(), symbol.0));
        }
        if let Some(action) = self.action {
            params.push(("action".to_string(), action.as_api_str().to_string()));
        }
        if let Some(partition) = self.partition_key {
            params.push(("partition-key".to_string(), partition));
        }
        if let Some(futures) = self.futures_symbol {
            params.push(("futures-symbol".to_string(), futures));
        }
        if let Some(offset) = self.page_offset {
            params.push(("page-offset".to_string(), offset.to_string()));
        }
        if let Some(per_page) = self.per_page {
            params.push(("per-page".to_string(), per_page.to_string()));
        }

        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::str::FromStr;

    // Test TransactionType serialization and deserialization
    #[test]
    fn test_transaction_type_serde() {
        assert_eq!(
            serde_json::to_string(&TransactionType::Trade).unwrap(),
            "\"Trade\""
        );
        assert_eq!(
            serde_json::to_string(&TransactionType::MoneyMovement).unwrap(),
            "\"Money Movement\""
        );
        assert_eq!(
            serde_json::to_string(&TransactionType::AdministrativeTransfer).unwrap(),
            "\"Administrative Transfer\""
        );
        assert_eq!(
            serde_json::to_string(&TransactionType::ReceiveDeliver).unwrap(),
            "\"Receive Deliver\""
        );

        // Test deserialization
        assert!(matches!(
            serde_json::from_str::<TransactionType>("\"Trade\"").unwrap(),
            TransactionType::Trade
        ));
        assert!(matches!(
            serde_json::from_str::<TransactionType>("\"Money Movement\"").unwrap(),
            TransactionType::MoneyMovement
        ));

        // Test unknown variant
        assert!(matches!(
            serde_json::from_str::<TransactionType>("\"UnknownType\"").unwrap(),
            TransactionType::Unknown
        ));
    }

    #[test]
    fn test_transaction_type_as_api_str() {
        assert_eq!(TransactionType::Trade.as_api_str(), "Trade");
        assert_eq!(
            TransactionType::MoneyMovement.as_api_str(),
            "Money Movement"
        );
        assert_eq!(
            TransactionType::AdministrativeTransfer.as_api_str(),
            "Administrative Transfer"
        );
        assert_eq!(
            TransactionType::ReceiveDeliver.as_api_str(),
            "Receive Deliver"
        );
        assert_eq!(TransactionType::Unknown.as_api_str(), "Unknown");
    }

    #[test]
    fn test_transaction_sub_type_serde() {
        assert_eq!(
            serde_json::to_string(&TransactionSubType::Dividend).unwrap(),
            "\"Dividend\""
        );
        assert_eq!(
            serde_json::to_string(&TransactionSubType::CashSettledExercise).unwrap(),
            "\"Cash Settled Exercise\""
        );
        assert_eq!(
            serde_json::to_string(&TransactionSubType::FullyPaidStockLendingIncome).unwrap(),
            "\"Fully Paid Stock Lending Income\""
        );

        // Test deserialization
        assert!(matches!(
            serde_json::from_str::<TransactionSubType>("\"Dividend\"").unwrap(),
            TransactionSubType::Dividend
        ));
        assert!(matches!(
            serde_json::from_str::<TransactionSubType>("\"Cash Settled Exercise\"").unwrap(),
            TransactionSubType::CashSettledExercise
        ));

        // Test unknown variant
        assert!(matches!(
            serde_json::from_str::<TransactionSubType>("\"UnknownSubType\"").unwrap(),
            TransactionSubType::Unknown
        ));
    }

    #[test]
    fn test_transaction_sub_type_as_api_str() {
        assert_eq!(TransactionSubType::Dividend.as_api_str(), "Dividend");
        assert_eq!(
            TransactionSubType::CashSettledExercise.as_api_str(),
            "Cash Settled Exercise"
        );
        assert_eq!(TransactionSubType::ACAT.as_api_str(), "ACAT");
        assert_eq!(
            TransactionSubType::FullyPaidStockLendingIncome.as_api_str(),
            "Fully Paid Stock Lending Income"
        );
    }

    #[test]
    fn test_transaction_lot_deserialization() {
        let json = json!({
            "lot-id": 12345,
            "symbol": "AAPL",
            "instrument-type": "Equity",
            "quantity": "100.00",
            "price": "150.25",
            "value": "15025.00"
        });

        let lot: TransactionLot = serde_json::from_value(json).unwrap();
        assert_eq!(lot.lot_id, Some(12345));
        assert_eq!(lot.symbol.as_ref().unwrap().0, "AAPL");
        assert_eq!(lot.quantity, Some(Decimal::from_str("100.00").unwrap()));
        assert_eq!(lot.price, Some(Decimal::from_str("150.25").unwrap()));
        assert_eq!(lot.value, Some(Decimal::from_str("15025.00").unwrap()));
    }

    #[test]
    fn test_transaction_lot_partial_deserialization() {
        let json = json!({
            "lot-id": 67890,
            "symbol": "SPY"
        });

        let lot: TransactionLot = serde_json::from_value(json).unwrap();
        assert_eq!(lot.lot_id, Some(67890));
        assert_eq!(lot.symbol.as_ref().unwrap().0, "SPY");
        assert!(lot.quantity.is_none());
        assert!(lot.price.is_none());
        assert!(lot.value.is_none());
    }

    #[test]
    fn test_total_fees_deserialization() {
        let json = json!({
            "total-fees": "2.50",
            "total-fees-effect": "Debit"
        });

        let fees: TotalFees = serde_json::from_value(json).unwrap();
        assert_eq!(fees.total_fees, Decimal::from_str("2.50").unwrap());
        assert!(matches!(fees.total_fees_effect, PriceEffect::Debit));
    }

    #[test]
    fn test_total_fees_credit_effect() {
        let json = json!({
            "total-fees": "0.00",
            "total-fees-effect": "Credit"
        });

        let fees: TotalFees = serde_json::from_value(json).unwrap();
        assert_eq!(fees.total_fees, Decimal::ZERO);
        assert!(matches!(fees.total_fees_effect, PriceEffect::Credit));
    }

    #[test]
    fn test_transaction_query_params_validate_conflicting_type_filters() {
        let mut params = TransactionQueryParams::default();
        params.transaction_type = Some(TransactionType::Trade);
        params.transaction_types = vec![TransactionType::MoneyMovement];

        let result = params.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransactionQueryError::ConflictingTypeFilters
        ));
    }

    #[test]
    fn test_transaction_query_params_validate_conflicting_date_filters() {
        let mut params = TransactionQueryParams::default();
        params.start_date = Some(chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap());
        params.start_at = Some(
            chrono::DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z").unwrap(),
        );

        let result = params.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransactionQueryError::ConflictingDateFilters
        ));
    }

    #[test]
    fn test_transaction_query_params_validate_success() {
        let mut params = TransactionQueryParams::default();
        params.transaction_types = vec![TransactionType::Trade];
        params.start_date = Some(chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap());

        let result = params.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_transaction_query_params_into_query_basic() {
        let mut params = TransactionQueryParams::default();
        params.sort = Some(SortOrder::Desc);
        params.transaction_type = Some(TransactionType::Trade);
        params.page_offset = Some(10);
        params.per_page = Some(50);

        let query = params.into_query();
        assert_eq!(query.len(), 4);
        assert!(query.contains(&("sort".to_string(), "Desc".to_string())));
        assert!(query.contains(&("type".to_string(), "Trade".to_string())));
        assert!(query.contains(&("page-offset".to_string(), "10".to_string())));
        assert!(query.contains(&("per-page".to_string(), "50".to_string())));
    }

    #[test]
    fn test_transaction_query_params_into_query_arrays() {
        let mut params = TransactionQueryParams::default();
        params.transaction_types = vec![TransactionType::Trade, TransactionType::MoneyMovement];
        params.sub_types = vec![TransactionSubType::Dividend, TransactionSubType::Fee];

        let query = params.into_query();

        // Count occurrences
        let types_count = query
            .iter()
            .filter(|(k, _)| k == "types[]")
            .count();
        let sub_types_count = query
            .iter()
            .filter(|(k, _)| k == "sub-type[]")
            .count();

        assert_eq!(types_count, 2);
        assert_eq!(sub_types_count, 2);
        assert!(query.contains(&("types[]".to_string(), "Trade".to_string())));
        assert!(query.contains(&("types[]".to_string(), "Money Movement".to_string())));
        assert!(query.contains(&("sub-type[]".to_string(), "Dividend".to_string())));
        assert!(query.contains(&("sub-type[]".to_string(), "Fee".to_string())));
    }

    #[test]
    fn test_transaction_query_params_into_query_dates() {
        let mut params = TransactionQueryParams::default();
        params.start_date = Some(chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap());
        params.end_date = Some(chrono::NaiveDate::from_ymd_opt(2023, 12, 31).unwrap());

        let query = params.into_query();
        assert!(query.contains(&("start-date".to_string(), "2023-01-01".to_string())));
        assert!(query.contains(&("end-date".to_string(), "2023-12-31".to_string())));
    }

    #[test]
    fn test_transaction_query_params_into_query_timestamps() {
        let mut params = TransactionQueryParams::default();
        let start = chrono::DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z").unwrap();
        let end = chrono::DateTime::parse_from_rfc3339("2023-12-31T23:59:59Z").unwrap();
        params.start_at = Some(start);
        params.end_at = Some(end);

        let query = params.into_query();
        assert!(query
            .iter()
            .any(|(k, v)| k == "start-at" && v == &start.to_rfc3339()));
        assert!(query
            .iter()
            .any(|(k, v)| k == "end-at" && v == &end.to_rfc3339()));
    }

    #[test]
    fn test_transaction_full_deserialization() {
        let json = json!({
            "id": 252640963,
            "account-number": "ACC123",
            "symbol": "AAPL",
            "instrument-type": "Equity",
            "underlying-symbol": "AAPL",
            "transaction-type": "Trade",
            "transaction-sub-type": "Dividend",
            "description": "Received dividend",
            "action": "Buy",
            "quantity": "100.00",
            "price": "150.25",
            "executed-at": "2023-07-28T14:30:00-04:00",
            "transaction-date": "2023-07-28",
            "value": "15025.00",
            "value-effect": "Debit",
            "net-value": "15023.50",
            "net-value-effect": "Debit",
            "cash-balance": "10000.00",
            "cash-balance-effect": "Credit",
            "commission": "1.00",
            "clearing-fees": "0.25",
            "regulatory-fees": "0.15",
            "occ-fees": "0.10",
            "proprietary-fees": "0.00",
            "other-fees": "0.00",
            "accrued-interest": "0.00",
            "settlement-date": "2023-07-30",
            "futures-symbol": null,
            "partition-key": "partition1",
            "instrument-id": 12345,
            "order-id": 98765,
            "execution-id": "exec123",
            "trade-transaction-type": "regular",
            "lots": [],
            "notes": "Test transaction",
            "tags": ["tag1", "tag2"],
            "is-estimated-fee": false
        });

        let tx: Transaction = serde_json::from_value(json).unwrap();
        assert_eq!(tx.id.0, 252640963);
        assert_eq!(tx.account_number.0, "ACC123");
        assert_eq!(tx.symbol.as_ref().unwrap().0, "AAPL");
        assert!(matches!(tx.transaction_type, TransactionType::Trade));
        assert!(matches!(tx.transaction_sub_type, TransactionSubType::Dividend));
        assert_eq!(tx.description, "Received dividend");
        assert!(tx.executed_at.is_some());
        assert_eq!(tx.quantity, Some(Decimal::from_str("100.00").unwrap()));
        assert_eq!(tx.price, Some(Decimal::from_str("150.25").unwrap()));
        assert_eq!(tx.value, Decimal::from_str("15025.00").unwrap());
        assert!(matches!(tx.value_effect, PriceEffect::Debit));
        assert_eq!(tx.commission, Some(Decimal::from_str("1.00").unwrap()));
        assert_eq!(tx.notes, Some("Test transaction".to_string()));
        assert_eq!(tx.tags.as_ref().unwrap().len(), 2);
        assert!(!tx.is_estimated_fee);
    }

    #[test]
    fn test_transaction_minimal_deserialization() {
        let json = json!({
            "id": 111,
            "account-number": "ACC456",
            "transaction-type": "Money Movement",
            "transaction-sub-type": "Deposit",
            "description": "Deposit",
            "transaction-date": "2023-01-01",
            "value": "1000.00",
            "value-effect": "Credit",
            "net-value": "1000.00",
            "net-value-effect": "Credit",
            "is-estimated-fee": false
        });

        let tx: Transaction = serde_json::from_value(json).unwrap();
        assert_eq!(tx.id.0, 111);
        assert!(tx.symbol.is_none());
        assert!(tx.instrument_type.is_none());
        assert!(tx.action.is_none());
        assert!(tx.quantity.is_none());
        assert!(tx.price.is_none());
        assert!(tx.executed_at.is_none());
        assert!(tx.commission.is_none());
        assert!(tx.notes.is_none());
        assert!(tx.tags.is_none());
        assert_eq!(tx.lots.len(), 0);
    }

    #[test]
    fn test_transaction_high_precision_decimals() {
        let json = json!({
            "id": 999,
            "account-number": "PREC123",
            "transaction-type": "Trade",
            "transaction-sub-type": "Assignment",
            "description": "High precision test",
            "transaction-date": "2023-01-01",
            "quantity": "123.123456789",
            "price": "999.999999999",
            "value": "123456.123456789",
            "value-effect": "Debit",
            "net-value": "123455.123456789",
            "net-value-effect": "Debit",
            "commission": "0.999999999",
            "is-estimated-fee": true
        });

        let tx: Transaction = serde_json::from_value(json).unwrap();
        assert_eq!(
            tx.quantity,
            Some(Decimal::from_str("123.123456789").unwrap())
        );
        assert_eq!(tx.price, Some(Decimal::from_str("999.999999999").unwrap()));
        assert_eq!(tx.value, Decimal::from_str("123456.123456789").unwrap());
        assert_eq!(
            tx.commission,
            Some(Decimal::from_str("0.999999999").unwrap())
        );
        assert!(tx.is_estimated_fee);
    }

    #[test]
    fn test_transaction_with_lots() {
        let json = json!({
            "id": 777,
            "account-number": "ACC789",
            "transaction-type": "Trade",
            "transaction-sub-type": "Assignment",
            "description": "Trade with lots",
            "transaction-date": "2023-06-15",
            "value": "5000.00",
            "value-effect": "Credit",
            "net-value": "4998.00",
            "net-value-effect": "Credit",
            "lots": [
                {
                    "lot-id": 1,
                    "symbol": "AAPL",
                    "instrument-type": "Equity",
                    "quantity": "50.00",
                    "price": "100.00",
                    "value": "5000.00"
                }
            ],
            "is-estimated-fee": false
        });

        let tx: Transaction = serde_json::from_value(json).unwrap();
        assert_eq!(tx.lots.len(), 1);
        assert_eq!(tx.lots[0].lot_id, Some(1));
        assert_eq!(tx.lots[0].symbol.as_ref().unwrap().0, "AAPL");
    }

    #[test]
    fn test_transaction_unknown_enum_variants() {
        let json = json!({
            "id": 888,
            "account-number": "ACC999",
            "transaction-type": "SomeFutureType",
            "transaction-sub-type": "SomeFutureSubType",
            "description": "Unknown types",
            "transaction-date": "2023-01-01",
            "value": "100.00",
            "value-effect": "None",
            "net-value": "100.00",
            "net-value-effect": "None",
            "is-estimated-fee": false
        });

        let tx: Transaction = serde_json::from_value(json).unwrap();
        assert!(matches!(tx.transaction_type, TransactionType::Unknown));
        assert!(matches!(
            tx.transaction_sub_type,
            TransactionSubType::Unknown
        ));
    }

    #[test]
    fn test_sort_order_as_api_str() {
        assert_eq!(SortOrder::Asc.as_api_str(), "Asc");
        assert_eq!(SortOrder::Desc.as_api_str(), "Desc");
    }
}
