use std::process;
use tastytrade_rs::TastyTrade;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let username = args.next().unwrap();
    let password = args.next().unwrap();
    let live = args.next().unwrap();

    let live = if live == "live" { true } else { false };

    let login_result = if live {
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

    for account in accounts {
        println!("Account: {}", account.number().0);
        match account.positions().await {
            Ok(positions) => {
                if positions.is_empty() {
                    println!("No positions found for this account.");
                } else {
                    println!("Positions:");
                    for position in positions {
                        println!("---");
                        println!("Symbol: {}", position.symbol.0);
                        println!("Instrument Type: {:?}", position.instrument_type);
                        println!("Underlying Symbol: {}", position.underlying_symbol.0);
                        println!(
                            "Quantity: {} (Direction: {:?})",
                            position.quantity, position.quantity_direction
                        );
                        println!("Close Price: {}", position.close_price);
                        println!("Average Open Price: {}", position.average_open_price);
                        println!("Multiplier: {}", position.multiplier);
                        println!("Cost Effect: {:?}", position.cost_effect);
                        println!("Is Suppressed: {}", position.is_suppressed);
                        println!("Is Frozen: {}", position.is_frozen);
                        println!("Restricted Quantity: {}", position.restricted_quantity);
                        println!(
                            "Realized Day Gain: {} ({})",
                            position.realized_day_gain, position.realized_day_gain_effect
                        );
                        println!(
                            "Realized Today: {} ({})",
                            position.realized_today, position.realized_today_effect
                        );
                        println!("Created At: {}", position.created_at);
                        println!("Updated At: {}", position.updated_at);
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Failed to retrieve positions for account {}: {}",
                    account.number().0,
                    e
                );
            }
        }
    }
}
