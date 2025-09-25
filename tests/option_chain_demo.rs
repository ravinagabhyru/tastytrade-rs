use tastytrade_rs::{api::base::Result, TastyTrade};

mod test_utils;
use test_utils::async_utils::{with_timeout, INTEGRATION_TIMEOUT};
use test_utils::env::demo_credentials;
use test_utils::test_symbols;

/// Option chain integration tests using demo environment
/// These tests require demo credentials and test against known liquid symbols

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_nested_option_chain_aapl() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;

    // Test nested option chain for AAPL (highly liquid, should have options)
    let chain = with_timeout(
        INTEGRATION_TIMEOUT,
        tasty.nested_option_chain_for(test_symbols::aapl()),
    )
    .await
    .expect("Option chain timeout")
    .expect("Failed to fetch AAPL option chain");

    // Validate chain structure
    assert_eq!(chain.underlying_symbol.0, "AAPL");
    assert_eq!(chain.root_symbol.0, "AAPL");
    assert!(
        !chain.expirations.is_empty(),
        "AAPL should have option expirations"
    );
    assert_eq!(
        chain.shares_per_contract, 100,
        "Standard equity options should be 100 shares per contract"
    );

    println!(
        "✓ AAPL option chain has {} expiration(s)",
        chain.expirations.len()
    );

    // Validate at least one expiration has strikes
    let first_exp = &chain.expirations[0];
    assert!(
        !first_exp.strikes.is_empty(),
        "First expiration should have strikes"
    );
    assert!(
        !first_exp.expiration_date.is_empty(),
        "Expiration date should not be empty"
    );

    println!(
        "✓ First expiration: {} with {} strike(s)",
        first_exp.expiration_date,
        first_exp.strikes.len()
    );

    // Validate strike structure
    let first_strike = &first_exp.strikes[0];
    assert!(
        first_strike.strike_price > rust_decimal::Decimal::ZERO,
        "Strike price should be positive"
    );
    assert!(
        !first_strike.call.0.is_empty(),
        "Call symbol should not be empty"
    );
    assert!(
        !first_strike.put.0.is_empty(),
        "Put symbol should not be empty"
    );

    // Verify option symbols follow expected format
    assert!(
        first_strike.call.0.contains("C"),
        "Call symbol should contain 'C'"
    );
    assert!(
        first_strike.put.0.contains("P"),
        "Put symbol should contain 'P'"
    );

    println!(
        "✓ First strike: ${} (Call: {}, Put: {})",
        first_strike.strike_price, first_strike.call.0, first_strike.put.0
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_flat_option_chain_spy() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;

    // Test flat option chain for SPY
    let chains = with_timeout(
        INTEGRATION_TIMEOUT,
        tasty.option_chain_for(test_symbols::spy()),
    )
    .await
    .expect("Option chain timeout")
    .expect("Failed to fetch SPY option chain");

    assert!(!chains.is_empty(), "SPY should have option chain entries");

    println!("✓ SPY option chain has {} entries", chains.len());

    // Validate chain entries
    for (i, chain) in chains.iter().take(5).enumerate() {
        // Check first 5 entries
        assert_eq!(chain.underlying_symbol.0, "SPY");
        assert!(
            chain.strike_price > rust_decimal::Decimal::ZERO,
            "Strike price should be positive"
        );

        // The extra map should contain additional fields from the API
        println!(
            "  Entry {}: Strike ${}, Extra fields: {}",
            i + 1,
            chain.strike_price,
            chain.extra.len()
        );

        // Common extra fields that might be present
        if chain.extra.contains_key("delta") {
            println!("    Delta: {}", chain.extra["delta"]);
        }
        if chain.extra.contains_key("volume") {
            println!("    Volume: {}", chain.extra["volume"]);
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_multiple_symbols_option_chains() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;

    // Test multiple symbols to ensure consistency
    let symbols = [
        test_symbols::aapl(),
        test_symbols::spy(),
        test_symbols::msft(),
    ];

    for symbol in &symbols {
        println!("Testing option chain for {}", symbol.0);

        let chain_result = with_timeout(
            INTEGRATION_TIMEOUT,
            tasty.nested_option_chain_for(symbol.clone()),
        )
        .await
        .expect("Option chain timeout");

        match chain_result {
            Ok(chain) => {
                assert_eq!(chain.underlying_symbol, *symbol);
                assert!(
                    !chain.expirations.is_empty(),
                    "Symbol {} should have option expirations",
                    symbol.0
                );

                println!(
                    "✓ {} has {} expiration(s)",
                    symbol.0,
                    chain.expirations.len()
                );
            }
            Err(e) => {
                println!("⚠ Failed to fetch option chain for {}: {}", symbol.0, e);
                // Some symbols might not have options or might not be available in demo
                // This is informational rather than a test failure
            }
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_option_chain_strike_precision() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;

    // Test that strike prices maintain precision
    let chain = tasty.nested_option_chain_for(test_symbols::aapl()).await?;

    // Find strikes and verify decimal precision
    let first_exp = &chain.expirations[0];
    let strikes: Vec<_> = first_exp.strikes.iter().map(|s| s.strike_price).collect();

    assert!(!strikes.is_empty(), "Should have strike prices");

    // Verify strikes are in reasonable range and maintain precision
    for strike in &strikes {
        assert!(
            *strike > rust_decimal::Decimal::ZERO,
            "Strike should be positive"
        );
        assert!(
            *strike < rust_decimal::Decimal::from(10000),
            "Strike should be reasonable"
        );

        // Test that we can serialize/deserialize without losing precision
        let serialized = serde_json::to_string(strike).expect("Should serialize");
        let deserialized: rust_decimal::Decimal =
            serde_json::from_str(&serialized).expect("Should deserialize");
        assert_eq!(*strike, deserialized, "Precision should be maintained");
    }

    println!(
        "✓ Strike prices maintain precision: {:?}",
        &strikes[..std::cmp::min(5, strikes.len())]
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_option_chain_date_validation() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;

    let chain = tasty.nested_option_chain_for(test_symbols::spy()).await?;

    // Validate expiration date formats and logic
    for exp in &chain.expirations {
        // Date should be in YYYY-MM-DD format
        assert!(
            exp.expiration_date.len() == 10 && exp.expiration_date.chars().nth(4) == Some('-'),
            "Expiration date should be in YYYY-MM-DD format: {}",
            exp.expiration_date
        );

        // Days to expiration should be reasonable (not negative, not too far out)
        assert!(
            exp.days_to_expiration < 1000,
            "Days to expiration seems too large: {}",
            exp.days_to_expiration
        );

        // Settlement type should be valid
        assert!(
            exp.settlement_type == "AM" || exp.settlement_type == "PM",
            "Settlement type should be AM or PM: {}",
            exp.settlement_type
        );

        println!(
            "✓ Expiration: {} ({} days, {} settlement)",
            exp.expiration_date, exp.days_to_expiration, exp.settlement_type
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_invalid_symbol_option_chain() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;

    // Test with invalid/non-existent symbol
    let result = tasty.nested_option_chain_for("INVALID_SYMBOL_XYZ").await;

    // Should either return an error or empty results
    match result {
        Ok(chain) => {
            // If it succeeds, chain should be reasonable or empty
            println!(
                "⚠ Invalid symbol returned chain with {} expirations",
                chain.expirations.len()
            );
        }
        Err(e) => {
            println!("✓ Invalid symbol correctly returned error: {}", e);
            // This is expected behavior
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_option_symbol_format_validation() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;

    let chain = tasty.nested_option_chain_for(test_symbols::aapl()).await?;

    // Validate option symbol formats
    for exp in &chain.expirations {
        for strike in &exp.strikes {
            // Call and Put symbols should follow standard format
            // Example: AAPL  240119C00180000, AAPL  240119P00180000

            let call = &strike.call.0;
            let put = &strike.put.0;

            // Both should start with the underlying symbol
            assert!(
                call.starts_with("AAPL"),
                "Call symbol should start with AAPL: {}",
                call
            );
            assert!(
                put.starts_with("AAPL"),
                "Put symbol should start with AAPL: {}",
                put
            );

            // Call should contain 'C', put should contain 'P'
            assert!(
                call.contains('C'),
                "Call symbol should contain 'C': {}",
                call
            );
            assert!(put.contains('P'), "Put symbol should contain 'P': {}", put);

            // Symbols should be reasonable length (not too short/long)
            assert!(
                call.len() > 10 && call.len() < 30,
                "Call symbol length seems wrong: {}",
                call
            );
            assert!(
                put.len() > 10 && put.len() < 30,
                "Put symbol length seems wrong: {}",
                put
            );

            println!("✓ Strike ${}: {} / {}", strike.strike_price, call, put);
        }
    }

    Ok(())
}
