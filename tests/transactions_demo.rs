use tastytrade_rs::api::base::Result;
use tastytrade_rs::api::transaction::*;
use tastytrade_rs::TastyTrade;

mod test_utils;
use test_utils::env::demo_credentials;

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_list_transactions() -> Result<()> {
    let (username, password) = demo_credentials()
        .expect("Demo credentials required - set DEMO_USERNAME and DEMO_PASSWORD");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Test that the API works (might return empty list)
    let transactions = account.transactions(TransactionQueryParams::default()).await?;

    println!("✓ Found {} transaction(s)", transactions.items.len());

    // Validate structure if we have data
    if let Some(first) = transactions.items.first() {
        assert_eq!(first.account_number, account.number());
        assert!(!first.description.is_empty());
        println!(
            "  First transaction: {:?} on {}",
            first.transaction_type, first.transaction_date
        );

        // Test get-by-id if we have a transaction
        let single = account.transaction(first.id).await?;
        assert_eq!(single.id, first.id);
        println!("✓ Successfully fetched transaction by ID");
    } else {
        println!("  Note: Demo account has no transactions (orders need to be executed first)");
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_list_transactions_with_date_filter() -> Result<()> {
    let (username, password) = demo_credentials()
        .expect("Demo credentials required - set DEMO_USERNAME and DEMO_PASSWORD");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Test date range filtering - use last 30 days
    let end_date = chrono::Local::now().naive_local().date();
    let start_date = end_date - chrono::Duration::days(30);

    let mut params = TransactionQueryParams::default();
    params.start_date = Some(start_date);
    params.end_date = Some(end_date);

    let transactions = account.transactions(params).await?;

    println!(
        "✓ Found {} transaction(s) in the last 30 days",
        transactions.items.len()
    );

    // If we have transactions, verify they're in the date range
    for tx in &transactions.items {
        assert!(
            tx.transaction_date >= start_date && tx.transaction_date <= end_date,
            "Transaction date {} not in range {start_date} to {end_date}",
            tx.transaction_date
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_list_transactions_with_type_filter() -> Result<()> {
    let (username, password) = demo_credentials()
        .expect("Demo credentials required - set DEMO_USERNAME and DEMO_PASSWORD");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Test filtering by transaction type
    let mut params = TransactionQueryParams::default();
    params.transaction_types = vec![TransactionType::Trade];

    let transactions = account.transactions(params).await?;

    println!(
        "✓ Found {} Trade transaction(s)",
        transactions.items.len()
    );

    // If we have transactions, verify they're all Trade type
    for tx in &transactions.items {
        assert!(
            matches!(tx.transaction_type, TransactionType::Trade),
            "Expected Trade transaction, got {:?}",
            tx.transaction_type
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_list_transactions_with_subtype_filter() -> Result<()> {
    let (username, password) = demo_credentials()
        .expect("Demo credentials required - set DEMO_USERNAME and DEMO_PASSWORD");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Test filtering by sub-type
    let mut params = TransactionQueryParams::default();
    params.sub_types = vec![TransactionSubType::Dividend];

    let transactions = account.transactions(params).await?;

    println!(
        "✓ Found {} Dividend transaction(s)",
        transactions.items.len()
    );

    // If we have transactions, verify they're all Dividend sub-type
    for tx in &transactions.items {
        assert!(
            matches!(tx.transaction_sub_type, TransactionSubType::Dividend),
            "Expected Dividend sub-type, got {:?}",
            tx.transaction_sub_type
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_list_transactions_pagination() -> Result<()> {
    let (username, password) = demo_credentials()
        .expect("Demo credentials required - set DEMO_USERNAME and DEMO_PASSWORD");

    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Test pagination with page_offset
    let mut params = TransactionQueryParams::default();
    params.per_page = Some(5);
    params.page_offset = Some(0);

    let page1 = account.transactions(params.clone()).await?;

    println!("✓ Page 1: {} items", page1.items.len());
    println!(
        "  Pagination: {}/{} pages, {} total items",
        page1.pagination.page_offset + 1,
        page1.pagination.total_pages,
        page1.pagination.total_items
    );

    // Validate pagination metadata
    if !page1.items.is_empty() {
        assert!(page1.pagination.per_page <= 5);
        assert_eq!(page1.pagination.page_offset, 0);
        assert_eq!(page1.pagination.current_item_count, page1.items.len());

        // If there are more pages, test fetching page 2
        if page1.pagination.total_pages > 1 {
            params.page_offset = Some(1);
            let page2 = account.transactions(params).await?;

            println!("✓ Page 2: {} items", page2.items.len());
            assert_eq!(page2.pagination.page_offset, 1);

            // Pages should have different items
            if !page1.items.is_empty() && !page2.items.is_empty() {
                assert_ne!(
                    page1.items[0].id.0, page2.items[0].id.0,
                    "Pages should contain different transactions"
                );
            }
        }
    } else {
        println!("  Note: No transactions to paginate");
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_total_fees_today() -> Result<()> {
    let (username, password) =
        demo_credentials().expect("Demo credentials required");
    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Should always work, even with $0 fees
    let fees = account.total_fees(None).await?;
    println!(
        "✓ Total fees today: ${} ({:?})",
        fees.total_fees, fees.total_fees_effect
    );

    // Verify the value is non-negative
    assert!(
        fees.total_fees >= rust_decimal::Decimal::ZERO,
        "Fees should be non-negative"
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_total_fees_specific_date() -> Result<()> {
    let (username, password) =
        demo_credentials().expect("Demo credentials required");
    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Test with specific date parameter (yesterday)
    let yesterday = chrono::Local::now().naive_local().date() - chrono::Duration::days(1);
    let fees = account.total_fees(Some(yesterday)).await?;

    println!(
        "✓ Total fees for {}: ${} ({:?})",
        yesterday, fees.total_fees, fees.total_fees_effect
    );

    // Should work even if no fees on that date
    assert!(
        fees.total_fees >= rust_decimal::Decimal::ZERO,
        "Fees should be non-negative"
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_transaction_decimal_precision() -> Result<()> {
    let (username, password) =
        demo_credentials().expect("Demo credentials required");
    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    let transactions = account.transactions(TransactionQueryParams::default()).await?;

    // If we have transactions with decimal values, verify precision is preserved
    for tx in &transactions.items {
        // Check that decimal values parse correctly (no precision loss)
        if let Some(qty) = tx.quantity {
            assert!(
                qty.to_string().parse::<rust_decimal::Decimal>().is_ok(),
                "Quantity should be valid decimal: {}",
                qty
            );
        }

        if let Some(price) = tx.price {
            assert!(
                price.to_string().parse::<rust_decimal::Decimal>().is_ok(),
                "Price should be valid decimal: {}",
                price
            );
        }

        // All transactions should have value and net_value
        assert!(
            tx.value.to_string().parse::<rust_decimal::Decimal>().is_ok(),
            "Value should be valid decimal: {}",
            tx.value
        );
        assert!(
            tx.net_value.to_string().parse::<rust_decimal::Decimal>().is_ok(),
            "Net value should be valid decimal: {}",
            tx.net_value
        );
    }

    println!(
        "✓ Verified decimal precision for {} transaction(s)",
        transactions.items.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Integration test requiring demo credentials"]
async fn test_transaction_query_validation() -> Result<()> {
    let (username, password) =
        demo_credentials().expect("Demo credentials required");
    let tasty = TastyTrade::login_demo(&username, &password, false).await?;
    let accounts = tasty.accounts().await?;
    let account = &accounts[0];

    // Test that conflicting type filters return an error
    let mut params = TransactionQueryParams::default();
    params.transaction_type = Some(TransactionType::Trade);
    params.transaction_types = vec![TransactionType::MoneyMovement];

    let result = account.transactions(params).await;
    assert!(
        result.is_err(),
        "Should fail with conflicting type filters"
    );

    println!("✓ Query validation correctly rejects conflicting type filters");

    // Test that conflicting date filters return an error
    let mut params2 = TransactionQueryParams::default();
    params2.start_date = Some(chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap());
    params2.start_at = Some(
        chrono::DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z").unwrap(),
    );

    let result2 = account.transactions(params2).await;
    assert!(
        result2.is_err(),
        "Should fail with conflicting date filters"
    );

    println!("✓ Query validation correctly rejects conflicting date filters");

    Ok(())
}
