use tastytrade_rs::{api::base::Result, TastyTrade};

mod test_utils;
use test_utils::async_utils::{with_timeout, INTEGRATION_TIMEOUT};
use test_utils::env::{demo_account_number, demo_credentials};

/// Demo login and account status integration tests
/// These tests require environment variables:
/// - DEMO_USERNAME: Demo account username
/// - DEMO_PASSWORD: Demo account password  
/// - TEST_ACCOUNT_NUMBER: Demo account number (optional)

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_login_and_accounts() -> Result<()> {
    let (username, password) = demo_credentials()
        .expect("Demo credentials required - set DEMO_USERNAME and DEMO_PASSWORD");

    // Test login to demo environment
    let tasty = with_timeout(
        INTEGRATION_TIMEOUT,
        TastyTrade::login_demo(&username, &password, false),
    )
    .await
    .expect("Login timeout")
    .expect("Failed to login to demo environment");

    // We're using login_demo so we should be in demo mode
    // Note: demo field is private, but we know we used login_demo

    // Fetch accounts
    let accounts = with_timeout(INTEGRATION_TIMEOUT, tasty.accounts())
        .await
        .expect("Accounts timeout")
        .expect("Failed to fetch accounts");

    // Should have at least one account in demo
    assert!(
        !accounts.is_empty(),
        "Demo environment should have at least one account"
    );

    println!(
        "✓ Successfully fetched {} account(s) from demo environment",
        accounts.len()
    );

    // Test account details
    let first_account = &accounts[0];
    let account_number = first_account.number();

    // Verify account number format (should be non-empty string)
    assert!(
        !account_number.0.is_empty(),
        "Account number should not be empty"
    );

    println!("✓ First account number: {}", account_number.0);

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_account_balance() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Test balance fetching
    let balance = with_timeout(INTEGRATION_TIMEOUT, account.balance())
        .await
        .expect("Balance timeout")
        .expect("Failed to fetch account balance");

    // Basic validation that balance data is reasonable
    assert_eq!(balance.account_number, account.number());

    // Balance fields should be valid decimals (not necessarily positive in demo)
    println!("✓ Cash balance: {}", balance.cash_balance);
    println!("✓ Net liquidating value: {}", balance.net_liquidating_value);
    println!("✓ Buying power: {}", balance.equity_buying_power);

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_account_positions() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Test positions fetching
    let positions = with_timeout(INTEGRATION_TIMEOUT, account.positions())
        .await
        .expect("Positions timeout")
        .expect("Failed to fetch account positions");

    // Demo account may or may not have positions
    println!("✓ Account has {} position(s)", positions.len());

    // If we have positions, validate structure
    for (i, position) in positions.iter().enumerate() {
        assert_eq!(position.account_number, account.number());
        assert!(
            !position.symbol.0.is_empty(),
            "Position symbol should not be empty"
        );

        println!(
            "  Position {}: {:?} {} shares of {}",
            i + 1,
            position.quantity_direction,
            position.quantity,
            position.symbol.0
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_live_orders() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Test live orders fetching
    let orders = with_timeout(INTEGRATION_TIMEOUT, account.live_orders())
        .await
        .expect("Orders timeout")
        .expect("Failed to fetch live orders");

    // Demo account likely has no live orders, but API should work
    println!("✓ Account has {} live order(s)", orders.len());

    // If we have live orders, validate structure
    for (i, order) in orders.iter().enumerate() {
        assert_eq!(order.account_number, account.number());
        assert!(
            !order.underlying_symbol.0.is_empty(),
            "Order symbol should not be empty"
        );

        println!(
            "  Order {}: {:?} {} of {} at ${}",
            i + 1,
            order.order_type,
            order.size,
            order.underlying_symbol.0,
            order.price
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials and specific account"]
async fn test_specific_demo_account() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let account_num = demo_account_number()
        .expect("Specific account test requires TEST_ACCOUNT_NUMBER environment variable");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;

    // Try to find the specific account
    let specific_account = tasty.account(account_num.as_str()).await?;

    match specific_account {
        Some(account) => {
            println!("✓ Found specific account: {}", account.number().0);

            // Test that we can fetch data for this specific account
            let balance = account.balance().await?;
            assert_eq!(balance.account_number.0, account_num);

            println!("✓ Successfully fetched balance for specific account");
        }
        None => {
            println!(
                "⚠ Specific account {} not found in demo environment",
                account_num
            );
            // This might be expected - the test account may not exist
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_login_failure() {
    // Test login with invalid credentials
    let result = TastyTrade::login_demo("invalid_user", "invalid_pass", false).await;

    // Should fail with an API error
    assert!(
        result.is_err(),
        "Login with invalid credentials should fail"
    );

    match result {
        Err(e) => {
            println!("✓ Expected login failure: {}", e);
            // Could check for specific error types/codes here
        }
        Ok(_) => panic!("Login with invalid credentials unexpectedly succeeded"),
    }
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_demo_session_token_usage() -> Result<()> {
    let (username, password) = demo_credentials().expect("Demo credentials required");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;

    // Session token field is private, but we can verify functionality by making requests

    // Make a request to verify token works
    let accounts = tasty.accounts().await?;
    assert!(
        !accounts.is_empty(),
        "Should be able to fetch accounts with session token"
    );

    println!("✓ Session token is valid and functional");

    Ok(())
}
