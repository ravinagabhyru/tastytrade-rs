use std::process;
use tastytrade_rs::TastyTrade;
use tastytrade_rs::api::oauth2::OAuth2Config;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Check for --oauth flag
    let tasty = if args.first().map(|s| s.as_str()) == Some("--oauth") {
        let live = args.get(1).map(|s| s.as_str()) == Some("live");
        run_oauth_login(live).await
    } else {
        run_legacy_login(args).await
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

async fn run_oauth_login(live: bool) -> TastyTrade {
    let client_id = std::env::var("TT_OAUTH_CLIENT_ID").unwrap_or_else(|_| {
        eprintln!("Error: TT_OAUTH_CLIENT_ID environment variable not set");
        eprintln!("Usage: accounts-status --oauth [live]");
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
    println!("Attempting OAuth2 login ({} environment)...", env_name);

    match TastyTrade::oauth2_from_refresh_token(config, &refresh_token, !live).await {
        Ok(client) => {
            println!("OAuth2 login successful!");
            client
        }
        Err(e) => {
            eprintln!("OAuth2 login failed: {}", e);
            process::exit(1);
        }
    }
}

#[allow(deprecated)]
async fn run_legacy_login(args: Vec<String>) -> TastyTrade {
    let mut args = args.into_iter();
    let username = match args.next() {
        Some(u) => u,
        None => {
            eprintln!("Error: Missing username argument.");
            eprintln!("Usage: accounts-status <username> <password> [live]");
            eprintln!("       accounts-status --oauth [live]");
            process::exit(1);
        }
    };
    let password = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Error: Missing password argument.");
            eprintln!("Usage: accounts-status <username> <password> [live]");
            process::exit(1);
        }
    };
    let live = args.next().map(|s| s == "live").unwrap_or(false);

    let env_name = if live { "production" } else { "demo" };
    println!("Attempting legacy session login ({} environment)...", env_name);

    let login_result = if live {
        TastyTrade::login(&username, &password, false).await
    } else {
        TastyTrade::login_demo(&username, &password, false).await
    };

    match login_result {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Login failed: {}", e);
            process::exit(1);
        }
    }
}
