use std::fmt;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::api::base::Result;
use crate::client::TastyTrade;

use super::base::{Items, Paginated};
use super::order::{DryRunResult, LiveOrderRecord, Order, OrderId, OrderPlacedResult, PriceEffect};
use super::position::FullPosition;
use super::transaction::{TotalFees, Transaction, TransactionId, TransactionQueryParams};

impl TastyTrade {
    pub async fn accounts(&self) -> Result<Vec<Account<'_>>> {
        let resp: Items<AccountInner> = self.get("/customers/me/accounts").await?;
        Ok(resp
            .items
            .into_iter()
            .map(|inner| Account { inner, tasty: self })
            .collect())
    }

    pub async fn account(
        &self,
        account_number: impl Into<AccountNumber>,
    ) -> Result<Option<Account<'_>>> {
        let account_number = account_number.into();
        let accounts = self.accounts().await?;
        for account in accounts {
            if account.inner.account.account_number == account_number {
                return Ok(Some(account));
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde(transparent)]
pub struct AccountNumber(pub String);

impl<T: AsRef<str>> From<T> for AccountNumber {
    fn from(value: T) -> Self {
        Self(value.as_ref().to_owned())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountDetails {
    pub account_number: AccountNumber,
    pub external_id: Option<String>,
    pub opened_at: String,
    pub nickname: String,
    pub account_type_name: String,
    pub day_trader_status: bool,
    pub is_firm_error: bool,
    pub is_firm_proprietary: bool,
    pub is_test_drive: Option<bool>,
    pub margin_or_cash: String,
    pub is_foreign: bool,
    pub funding_date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountInner {
    pub account: AccountDetails,
    pub authority_level: String,
}

pub struct Account<'t> {
    pub(crate) inner: AccountInner,
    tasty: &'t TastyTrade,
}

impl<'t> Account<'t> {
    pub fn number(&self) -> AccountNumber {
        self.inner.account.account_number.clone()
    }

    pub async fn balance(&self) -> Result<Balance> {
        let resp = self
            .tasty
            .get(&format!(
                "/accounts/{}/balances",
                self.inner.account.account_number.0
            ))
            .await?;
        Ok(resp)
    }

    pub async fn balance_snapshot(
        &self,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
        tod: SnapshotTimeOfDay,
        page_offset: usize,
    ) -> Result<Paginated<BalanceSnapshot>> {
        let resp: Paginated<BalanceSnapshot> = self
            .tasty
            .get_with_query(
                &format!(
                    "/accounts/{}/balance-snapshots",
                    self.inner.account.account_number.0
                ),
                &[
                    ("start-date", &start_date.format("%Y-%m-%d").to_string()),
                    ("end-date", &end_date.format("%Y-%m-%d").to_string()),
                    ("page-offset", &page_offset.to_string()),
                    ("time-of-day", &tod.to_string()),
                ],
            )
            .await?;
        Ok(resp)
    }

    pub async fn positions(&self) -> Result<Vec<FullPosition>> {
        let resp: Items<FullPosition> = self
            .tasty
            .get(&format!(
                "/accounts/{}/positions",
                self.inner.account.account_number.0
            ))
            .await?;
        Ok(resp.items)
    }

    pub async fn live_orders(&self) -> Result<Vec<LiveOrderRecord>> {
        let resp: Items<LiveOrderRecord> = self
            .tasty
            .get(&format!(
                "/accounts/{}/orders/live",
                self.inner.account.account_number.0
            ))
            .await?;
        Ok(resp.items)
    }

    pub async fn dry_run(&self, order: &Order) -> Result<DryRunResult> {
        let resp: DryRunResult = self
            .tasty
            .post(
                &format!(
                    "/accounts/{}/orders/dry-run",
                    self.inner.account.account_number.0
                ),
                order,
            )
            .await?;
        Ok(resp)
    }

    pub async fn place_order(&self, order: &Order) -> Result<OrderPlacedResult> {
        let resp: OrderPlacedResult = self
            .tasty
            .post(
                &format!("/accounts/{}/orders", self.inner.account.account_number.0),
                order,
            )
            .await?;
        Ok(resp)
    }

    pub async fn cancel_order(&self, id: OrderId) -> Result<LiveOrderRecord> {
        self.tasty
            .delete(&format!(
                "/accounts/{}/orders/{}",
                self.inner.account.account_number.0, id.0
            ))
            .await
    }

    /// List transactions with optional filters
    pub async fn transactions(
        &self,
        params: TransactionQueryParams,
    ) -> Result<Paginated<Transaction>> {
        params.validate()?;
        let query_params = params.into_query();
        let query_refs: Vec<(&str, &str)> = query_params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        self.tasty
            .get_with_query(
                &format!(
                    "/accounts/{}/transactions",
                    self.inner.account.account_number.0
                ),
                &query_refs,
            )
            .await
    }

    /// Get a single transaction by ID
    pub async fn transaction(&self, id: TransactionId) -> Result<Transaction> {
        self.tasty
            .get(&format!(
                "/accounts/{}/transactions/{}",
                self.inner.account.account_number.0, id.0
            ))
            .await
    }

    /// Get total fees for a specific date (defaults to today)
    pub async fn total_fees(&self, date: Option<chrono::NaiveDate>) -> Result<TotalFees> {
        let query = if let Some(d) = date {
            vec![("date", d.format("%Y-%m-%d").to_string())]
        } else {
            vec![]
        };

        let query_refs: Vec<(&str, &str)> = query
            .iter()
            .map(|(k, v)| (*k, v.as_ref()))
            .collect();

        self.tasty
            .get_with_query(
                &format!(
                    "/accounts/{}/transactions/total-fees",
                    self.inner.account.account_number.0
                ),
                &query_refs,
            )
            .await
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Balance {
    pub account_number: AccountNumber,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub cash_balance: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub long_equity_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub short_equity_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub long_derivative_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub short_derivative_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub long_futures_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub short_futures_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub long_futures_derivative_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub short_futures_derivative_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub long_margineable_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub short_margineable_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub margin_equity: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub equity_buying_power: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub derivative_buying_power: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub day_trading_buying_power: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub futures_margin_requirement: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub available_trading_funds: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub maintenance_requirement: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub maintenance_call_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub reg_t_call_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub day_trading_call_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub day_equity_call_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub net_liquidating_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub cash_available_to_withdraw: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub day_trade_excess: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub pending_cash: Decimal,
    pub pending_cash_effect: PriceEffect,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub pending_margin_interest: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub effective_cryptocurrency_buying_power: Decimal,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BalanceSnapshot {
    pub account_number: AccountNumber,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub cash_balance: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub long_equity_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub short_equity_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub long_derivative_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub short_derivative_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub long_futures_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub short_futures_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub long_futures_derivative_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub short_futures_derivative_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub long_margineable_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub short_margineable_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub margin_equity: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub equity_buying_power: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub derivative_buying_power: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub day_trading_buying_power: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub futures_margin_requirement: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub available_trading_funds: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub maintenance_requirement: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub maintenance_call_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub reg_t_call_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub day_trading_call_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub day_equity_call_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub net_liquidating_value: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub cash_available_to_withdraw: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub day_trade_excess: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub pending_cash: Decimal,
    pub pending_cash_effect: PriceEffect,
    pub snapshot_date: chrono::NaiveDate,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SnapshotTimeOfDay {
    EOD,
    BOD,
}

impl fmt::Display for SnapshotTimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn test_account_number_from_str() {
        let account_number = AccountNumber::from("12345");
        assert_eq!(account_number.0, "12345");
    }

    #[test]
    fn test_account_number_from_string() {
        let account_number = AccountNumber::from("ABCDE".to_string());
        assert_eq!(account_number.0, "ABCDE");
    }

    #[test]
    fn test_account_number_ordering_and_equality() {
        let acc1 = AccountNumber::from("AAA123");
        let acc2 = AccountNumber::from("AAA123");
        let acc3 = AccountNumber::from("BBB456");

        assert_eq!(acc1, acc2);
        assert_ne!(acc1, acc3);
        assert!(acc1 < acc3); // AAA123 < BBB456 alphabetically
    }

    #[test]
    fn test_snapshot_time_of_day_display() {
        assert_eq!(format!("{}", SnapshotTimeOfDay::EOD), "EOD");
        assert_eq!(format!("{}", SnapshotTimeOfDay::BOD), "BOD");
    }

    #[test]
    fn test_account_details_deserialization() {
        let json = json!({
            "account-number": "ACC123",
            "external-id": "EXT456",
            "opened-at": "2023-01-01T00:00:00Z",
            "nickname": "Main Account",
            "account-type-name": "Individual",
            "day-trader-status": true,
            "is-firm-error": false,
            "is-firm-proprietary": false,
            "is-test-drive": true,
            "margin-or-cash": "Margin",
            "is-foreign": false,
            "funding-date": "2023-01-01"
        });

        let account_details: AccountDetails = serde_json::from_value(json).unwrap();
        assert_eq!(account_details.account_number.0, "ACC123");
        assert_eq!(account_details.external_id, Some("EXT456".to_string()));
        assert_eq!(account_details.opened_at, "2023-01-01T00:00:00Z");
        assert_eq!(account_details.nickname, "Main Account");
        assert_eq!(account_details.account_type_name, "Individual");
        assert!(account_details.day_trader_status);
        assert!(!account_details.is_firm_error);
        assert!(!account_details.is_firm_proprietary);
        assert_eq!(account_details.is_test_drive, Some(true));
        assert_eq!(account_details.margin_or_cash, "Margin");
        assert!(!account_details.is_foreign);
        assert_eq!(account_details.funding_date, Some("2023-01-01".to_string()));
    }

    #[test]
    fn test_account_inner_deserialization() {
        let json = json!({
            "account": {
                "account-number": "ACC789",
                "external-id": null,
                "opened-at": "2023-01-01T00:00:00Z",
                "nickname": "Test Account",
                "account-type-name": "Individual",
                "day-trader-status": false,
                "is-firm-error": false,
                "is-firm-proprietary": false,
                "is-test-drive": null,
                "margin-or-cash": "Cash",
                "is-foreign": false,
                "funding-date": null
            },
            "authority-level": "Owner"
        });

        let account_inner: AccountInner = serde_json::from_value(json).unwrap();
        assert_eq!(account_inner.account.account_number.0, "ACC789");
        assert_eq!(account_inner.account.nickname, "Test Account");
        assert!(!account_inner.account.day_trader_status);
        assert_eq!(account_inner.authority_level, "Owner");
    }

    #[test]
    fn test_balance_deserialization() {
        let json = json!({
            "account-number": "ACC123",
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
        });

        let balance: Balance = serde_json::from_value(json).unwrap();
        assert_eq!(balance.account_number.0, "ACC123");
        assert_eq!(balance.cash_balance, Decimal::from_str("10000.50").unwrap());
        assert_eq!(
            balance.long_equity_value,
            Decimal::from_str("25000.75").unwrap()
        );
        assert_eq!(
            balance.short_derivative_value,
            Decimal::from_str("1000.00").unwrap()
        );
        assert!(matches!(balance.pending_cash_effect, PriceEffect::None));
        assert_eq!(balance.updated_at, "2023-01-01T12:00:00Z");
    }

    #[test]
    fn test_balance_snapshot_deserialization() {
        let json = json!({
            "account-number": "ACC456",
            "cash-balance": "15000.25",
            "long-equity-value": "35000.50",
            "short-equity-value": "0.00",
            "long-derivative-value": "7500.00",
            "short-derivative-value": "2500.00",
            "long-futures-value": "0.00",
            "short-futures-value": "0.00",
            "long-futures-derivative-value": "0.00",
            "short-futures-derivative-value": "0.00",
            "long-margineable-value": "42500.50",
            "short-margineable-value": "2500.00",
            "margin-equity": "55000.25",
            "equity-buying-power": "110000.50",
            "derivative-buying-power": "55000.25",
            "day-trading-buying-power": "220000.50",
            "futures-margin-requirement": "0.00",
            "available-trading-funds": "55000.25",
            "maintenance-requirement": "10000.00",
            "maintenance-call-value": "0.00",
            "reg-t-call-value": "0.00",
            "day-trading-call-value": "0.00",
            "day-equity-call-value": "0.00",
            "net-liquidating-value": "55000.25",
            "cash-available-to-withdraw": "15000.25",
            "day-trade-excess": "210000.50",
            "pending-cash": "0.00",
            "pending-cash-effect": "Credit",
            "snapshot-date": "2023-01-01"
        });

        let balance_snapshot: BalanceSnapshot = serde_json::from_value(json).unwrap();
        assert_eq!(balance_snapshot.account_number.0, "ACC456");
        assert_eq!(
            balance_snapshot.cash_balance,
            Decimal::from_str("15000.25").unwrap()
        );
        assert_eq!(
            balance_snapshot.net_liquidating_value,
            Decimal::from_str("55000.25").unwrap()
        );
        assert!(matches!(
            balance_snapshot.pending_cash_effect,
            PriceEffect::Credit
        ));

        // Test date parsing
        use chrono::NaiveDate;
        let expected_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        assert_eq!(balance_snapshot.snapshot_date, expected_date);
    }

    #[test]
    fn test_snapshot_time_of_day_serde() {
        // Test serialization
        assert_eq!(
            serde_json::to_string(&SnapshotTimeOfDay::EOD).unwrap(),
            "\"EOD\""
        );
        assert_eq!(
            serde_json::to_string(&SnapshotTimeOfDay::BOD).unwrap(),
            "\"BOD\""
        );

        // Test deserialization
        let eod: SnapshotTimeOfDay = serde_json::from_str("\"EOD\"").unwrap();
        let bod: SnapshotTimeOfDay = serde_json::from_str("\"BOD\"").unwrap();

        assert!(matches!(eod, SnapshotTimeOfDay::EOD));
        assert!(matches!(bod, SnapshotTimeOfDay::BOD));
    }

    #[test]
    fn test_balance_arbitrary_precision_fields() {
        // Test that all decimal fields can handle high precision
        let json = json!({
            "account-number": "PREC123",
            "cash-balance": "12345.123456789",
            "long-equity-value": "98765.987654321",
            "short-equity-value": "0.000000001",
            "long-derivative-value": "5555.555555555",
            "short-derivative-value": "1111.111111111",
            "long-futures-value": "0.123456789",
            "short-futures-value": "0.987654321",
            "long-futures-derivative-value": "333.333333333",
            "short-futures-derivative-value": "666.666666666",
            "long-margineable-value": "77777.777777777",
            "short-margineable-value": "22222.222222222",
            "margin-equity": "99999.999999999",
            "equity-buying-power": "199999.999999998",
            "derivative-buying-power": "99999.999999999",
            "day-trading-buying-power": "399999.999999996",
            "futures-margin-requirement": "1234.567890123",
            "available-trading-funds": "88888.888888888",
            "maintenance-requirement": "11111.111111111",
            "maintenance-call-value": "0.000000000",
            "reg-t-call-value": "0.000000000",
            "day-trading-call-value": "0.000000000",
            "day-equity-call-value": "0.000000000",
            "net-liquidating-value": "123456.123456789",
            "cash-available-to-withdraw": "12345.123456789",
            "day-trade-excess": "388888.876543210",
            "pending-cash": "0.000000001",
            "pending-cash-effect": "Debit",
            "pending-margin-interest": "1.234567890",
            "effective-cryptocurrency-buying-power": "0.000000000",
            "updated-at": "2023-01-01T12:00:00Z"
        });

        let balance: Balance = serde_json::from_value(json).unwrap();

        // Test high precision preservation
        assert_eq!(
            balance.cash_balance,
            Decimal::from_str("12345.123456789").unwrap()
        );
        assert_eq!(
            balance.long_equity_value,
            Decimal::from_str("98765.987654321").unwrap()
        );
        assert_eq!(
            balance.short_equity_value,
            Decimal::from_str("0.000000001").unwrap()
        );
        assert_eq!(
            balance.net_liquidating_value,
            Decimal::from_str("123456.123456789").unwrap()
        );
        assert_eq!(
            balance.pending_margin_interest,
            Decimal::from_str("1.234567890").unwrap()
        );
    }

    // Note: We can't easily test the selection logic without mocking the HTTP calls
    // That would require integration tests marked with #[ignore]
}
