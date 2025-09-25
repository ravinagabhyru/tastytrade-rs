# tastytrade-rs

Rust library for stock market trading through tastytrade's API. Provides comprehensive access to:

- **Authentication** and session management (production and sandbox)
- **Account management** and portfolio operations
- **Order placement**, modification, and management
- **Real-time market data** streaming via DxLink protocol
- **Options chains** and instrument data
- **Quote streaming** with Greeks, trades, and market events

## Quick Start

### Building

```bash
# Build the library
cargo build

# Run tests
cargo test
```

### Running Examples

The `examples/` directory contains several examples demonstrating different features:

```bash
# Basic login test
cargo run --example test_login <username> <password>

# Account status and balances
cargo run --example accounts-status <username> <password> <demo/live>

# Real-time quote streaming
cargo run --example quote-streaming <username> <password> [live]

# REST market data snapshot with nearest strikes
cargo run --example market-data <username> <password> <demo/live> <symbol>

# Options chains with live Greeks data
cargo run --example option-chains <username> <password> <demo/live> <symbol> [options]
```

#### Options Chain Example

The options chain example supports filtering and live market data:

```bash
# Basic options chain
cargo run --example option-chains myuser mypass demo AAPL

# With live streaming data (quotes + Greeks)
cargo run --example option-chains myuser mypass demo AAPL --with-streaming

# With filters and streaming
cargo run --example option-chains myuser mypass demo SPX --strike-range 6590-6620 --max-dte 45 --with-streaming
```

**Options:**
- `--strike-range min-max` - Filter by strike price range
- `--max-dte days` - Filter by maximum days to expiration
- `--with-streaming` - Fetch live quotes and Greeks via DxLink

#### Market Data Example

The market data example calls the REST `/market-data/by-type` endpoint to retrieve the underlying quote along with a five-strike window (calls and puts) around the nearest expiration.

```bash
cargo run --example market-data myuser mypass demo AAPL
```

# Library Usage Example

```rust
    let tasty = TastyTrade::login("username", "password", false)
        .await
        .unwrap();

    let account = tasty.account("ABC12345")
        .await
        .unwrap()
        .unwrap();
    println!("{:#?}", account.balance().await);
    println!("{:#?}", account.positions().await);
    println!("{:#?}", account.live_orders().await);

    let order_leg = OrderLegBuilder::default()
        .instrument_type(InstrumentType::Equity)
        .symbol("AAPL")
        .quantity(1u64)
        .action(Action::BuyToOpen)
        .build()
        .unwrap();

    let order = OrderBuilder::default()
        .time_in_force(TimeInForce::GTC)
        .order_type(OrderType::Limit)
        .price(dec!(170.0))
        .price_effect(PriceEffect::Debit)
        .legs(vec![order_leg])
        .build()
        .unwrap();

    let dry_result = account.dry_run(&order).await;

    println!("{dry_result:#?}");
    // Outputs:
    // DryRunResult {
    //     order: DryRunRecord {
    //         account_number: AccountNumber(
    //             "ABC12345",
    //         ),
    //         time_in_force: GTC,
    //         order_type: Limit,
    //         size: 1,
    //         underlying_symbol: Symbol(
    //             "AAPL",
    //         ),
    //         price: 170.0,
    //         price_effect: Debit,
    //         status: Received,
    //         cancellable: true,
    //         editable: true,
    //         edited: false,
    //         legs: [
    //             OrderLeg {
    //                 instrument_type: Equity,
    //                 symbol: Symbol(
    //                     "AAPL",
    //                 ),
    //                 quantity: 1,
    //                 action: BuyToOpen,
    //             },
    //         ],
    //     },
    //     warnings: [],
    //     buying_power_effect: BuyingPowerEffect {
    //         change_in_margin_requirement: 85.0,
    //         change_in_margin_requirement_effect: Debit,
    //         change_in_buying_power: 85.001,
    //         change_in_buying_power_effect: Debit,
    //         current_buying_power: 562.5,
    //         current_buying_power_effect: Credit,
    //         impact: 85.001,
    //         effect: Debit,
    //     },
    //     fee_calculation: FeeCalculation {
    //         total_fees: 0.001,
    //         total_fees_effect: Debit,
    //     },
    // },
```
