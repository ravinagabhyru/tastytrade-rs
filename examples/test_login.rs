use std::process;
use tastytrade_rs::TastyTrade;
use tastytrade_rs::api::oauth2::OAuth2Config;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let live = args.first().map(|s| s.as_str()) == Some("live");

    let client_id = std::env::var("TT_OAUTH_CLIENT_ID").unwrap_or_else(|_| {
        eprintln!("Error: TT_OAUTH_CLIENT_ID environment variable not set");
        eprintln!("Usage: test_login [live]");
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

    match TastyTrade::from_refresh_token(config, &refresh_token, !live).await {
        Ok(client) => {
            println!("Login successful!");
            match client.accounts().await {
                Ok(accounts) => println!("Found {} account(s)", accounts.len()),
                Err(e) => eprintln!("Warning: Could not fetch accounts: {}", e),
            }
        }
        Err(e) => {
            eprintln!("Login failed: {}", e);
            process::exit(1);
        }
    }
}
