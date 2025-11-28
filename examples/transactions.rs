use std::process;
use tastytrade_rs::api::oauth2::OAuth2Config;
use tastytrade_rs::api::transaction::TransactionQueryParams;
use tastytrade_rs::TastyTrade;

fn get_oauth_config() -> (OAuth2Config, String) {
    let client_id = std::env::var("TT_OAUTH_CLIENT_ID").unwrap_or_else(|_| {
        eprintln!("Error: TT_OAUTH_CLIENT_ID environment variable not set");
        eprintln!("Required env vars: TT_OAUTH_CLIENT_ID, TT_OAUTH_CLIENT_SECRET, TT_OAUTH_REFRESH_TOKEN");
        process::exit(1);
    });
    let client_secret = std::env::var("TT_OAUTH_CLIENT_SECRET").unwrap_or_else(|_| {
        eprintln!("Error: TT_OAUTH_CLIENT_SECRET environment variable not set");
        process::exit(1);
    });
    let redirect_uri = std::env::var("TT_OAUTH_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost".to_string());
    let refresh_token = std::env::var("TT_OAUTH_REFRESH_TOKEN").unwrap_or_else(|_| {
        eprintln!("Error: TT_OAUTH_REFRESH_TOKEN environment variable not set");
        process::exit(1);
    });

    let config = OAuth2Config {
        client_id,
        client_secret,
        redirect_uri,
        scopes: vec!["read".to_string()],
    };
    (config, refresh_token)
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut live = false;
    let mut limit: Option<usize> = None;
    let mut days: Option<i64> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "live" => {
                live = true;
                i += 1;
            }
            "--limit" => {
                if i + 1 < args.len() {
                    limit = args[i + 1].parse().ok();
                    if limit.is_none() {
                        eprintln!("Error: Invalid limit value: {}", args[i + 1]);
                        process::exit(1);
                    }
                    i += 2;
                } else {
                    eprintln!("Error: --limit requires a number argument");
                    process::exit(1);
                }
            }
            "--days" => {
                if i + 1 < args.len() {
                    days = args[i + 1].parse().ok();
                    if days.is_none() {
                        eprintln!("Error: Invalid days value: {}", args[i + 1]);
                        process::exit(1);
                    }
                    i += 2;
                } else {
                    eprintln!("Error: --days requires a number argument");
                    process::exit(1);
                }
            }
            _ => {
                eprintln!("Error: Unknown argument: {}", args[i]);
                eprintln!("Usage: transactions [live] [--limit N] [--days N]");
                process::exit(1);
            }
        }
    }

    let (config, refresh_token) = get_oauth_config();
    let env_name = if live { "production" } else { "demo" };
    println!("Logging in ({} environment)...", env_name);

    let tasty = match TastyTrade::from_refresh_token(config, &refresh_token, !live).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Login failed: {}", e);
            process::exit(1);
        }
    };

    let accounts = match tasty.accounts().await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to retrieve accounts: {}", e);
            process::exit(1);
        }
    };

    println!("Number of Accounts: {}", accounts.len());
    println!();

    for account in accounts {
        println!("================================================================================");
        println!("Account: {}", account.number().0);
        println!("================================================================================");

        // Get total fees for today
        match account.total_fees(None).await {
            Ok(fees) => {
                println!("Total Fees Today: ${} ({:?})", fees.total_fees, fees.total_fees_effect);
            }
            Err(e) => {
                eprintln!("Failed to retrieve total fees: {}", e);
            }
        }

        println!();

        // Build query parameters
        let mut params = TransactionQueryParams::default();

        if let Some(per_page) = limit {
            params.per_page = Some(per_page);
        }

        if let Some(d) = days {
            let end_date = chrono::Local::now().naive_local().date();
            let start_date = end_date - chrono::Duration::days(d);
            params.start_date = Some(start_date);
            params.end_date = Some(end_date);
        }

        // List transactions
        match account.transactions(params).await {
            Ok(result) => {
                if result.items.is_empty() {
                    println!("No transactions found.");
                    if days.is_none() {
                        println!("Note: Demo accounts may not have transactions unless orders have been executed.");
                        println!("Tip: Use --days N to search the last N days.");
                    }
                } else {
                    println!("Recent Transactions ({} found):", result.items.len());
                    println!();

                    for tx in result.items {
                        println!("---");
                        println!("ID: {}", tx.id.0);
                        println!("Date: {}", tx.transaction_date);
                        println!("Type: {:?} ({:?})", tx.transaction_type, tx.transaction_sub_type);

                        if let Some(symbol) = &tx.symbol {
                            println!("Symbol: {}", symbol.0);
                        }

                        if let Some(action) = &tx.action {
                            print!("Action: {:?}", action);
                            if let Some(qty) = tx.quantity {
                                print!(" ({} shares)", qty);
                            }
                            println!();
                        }

                        if let Some(price) = tx.price {
                            println!("Price: ${}", price);
                        }

                        println!("Description: {}", tx.description);
                        println!("Amount: ${} ({:?})", tx.net_value, tx.net_value_effect);

                        if let Some(commission) = tx.commission {
                            if commission > rust_decimal::Decimal::ZERO {
                                println!("Commission: ${}", commission);
                            }
                        }

                        if let Some(balance) = tx.cash_balance {
                            println!("Cash Balance After: ${}", balance);
                        }
                    }

                    println!();
                    println!("Pagination: Page {}/{} (showing {} of {} total)",
                        result.pagination.page_offset + 1,
                        result.pagination.total_pages,
                        result.pagination.current_item_count,
                        result.pagination.total_items
                    );
                }
            }
            Err(e) => {
                eprintln!("Failed to retrieve transactions: {}", e);
            }
        }

        println!();
    }
}
