use std::process;
use tastytrade_rs::api::transaction::TransactionQueryParams;
use tastytrade_rs::TastyTrade;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let username = match args.next() {
        Some(u) => u,
        None => {
            eprintln!("Error: Missing username argument.");
            eprintln!("Usage: transactions <username> <password> <demo/live> [--limit N] [--days N]");
            process::exit(1);
        }
    };
    let password = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Error: Missing password argument.");
            eprintln!("Usage: transactions <username> <password> <demo/live> [--limit N] [--days N]");
            process::exit(1);
        }
    };
    let env = match args.next() {
        Some(e) => e,
        None => {
            eprintln!("Error: Missing environment argument.");
            eprintln!("Usage: transactions <username> <password> <demo/live> [--limit N] [--days N]");
            process::exit(1);
        }
    };

    // Parse optional arguments
    let mut limit: Option<usize> = None;
    let mut days: Option<i64> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--limit" => {
                limit = match args.next() {
                    Some(n) => match n.parse() {
                        Ok(num) => Some(num),
                        Err(_) => {
                            eprintln!("Error: Invalid limit value: {}", n);
                            process::exit(1);
                        }
                    },
                    None => {
                        eprintln!("Error: --limit requires a number argument");
                        process::exit(1);
                    }
                };
            }
            "--days" => {
                days = match args.next() {
                    Some(n) => match n.parse() {
                        Ok(num) => Some(num),
                        Err(_) => {
                            eprintln!("Error: Invalid days value: {}", n);
                            process::exit(1);
                        }
                    },
                    None => {
                        eprintln!("Error: --days requires a number argument");
                        process::exit(1);
                    }
                };
            }
            _ => {
                eprintln!("Error: Unknown argument: {}", arg);
                eprintln!("Usage: transactions <username> <password> <demo/live> [--limit N] [--days N]");
                process::exit(1);
            }
        }
    }

    let is_live = match env.as_str() {
        "live" => true,
        "demo" => false,
        _ => {
            eprintln!("Error: Environment must be either 'demo' or 'live', got: {}", env);
            process::exit(1);
        }
    };

    let login_result = if is_live {
        TastyTrade::login(&username, &password, false).await
    } else {
        TastyTrade::login_demo(&username, &password, false).await
    };

    let tasty = match login_result {
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
