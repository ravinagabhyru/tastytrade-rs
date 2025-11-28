use std::process;
use tastytrade_rs::TastyTrade;
use tastytrade_rs::api::oauth2::OAuth2Config;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let live = args.first().map(|s| s.as_str()) == Some("live");

    let client_id = std::env::var("TT_OAUTH_CLIENT_ID").unwrap_or_else(|_| {
        eprintln!("Error: TT_OAUTH_CLIENT_ID environment variable not set");
        eprintln!("Usage: accounts-status [live]");
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

    let env_name = if live { "production" } else { "demo" };
    println!("Attempting login ({} environment)...", env_name);

    let tasty = match TastyTrade::from_refresh_token(config, &refresh_token, !live).await {
        Ok(client) => {
            println!("Login successful!");
            client
        }
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
