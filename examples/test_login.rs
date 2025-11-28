use std::process;
use tastytrade_rs::TastyTrade;
use tastytrade_rs::api::oauth2::OAuth2Config;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Check for --oauth flag
    if args.first().map(|s| s.as_str()) == Some("--oauth") {
        run_oauth_login().await;
    } else {
        run_legacy_login(args).await;
    }
}

async fn run_oauth_login() {
    let client_id = std::env::var("TT_OAUTH_CLIENT_ID").unwrap_or_else(|_| {
        eprintln!("Error: TT_OAUTH_CLIENT_ID environment variable not set");
        eprintln!("Usage: test_login --oauth");
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

    println!("Attempting OAuth2 login (demo environment)...");
    match TastyTrade::oauth2_from_refresh_token(config, &refresh_token, true).await {
        Ok(client) => {
            println!("OAuth2 login successful!");
            // Verify by fetching accounts
            match client.accounts().await {
                Ok(accounts) => println!("Found {} account(s)", accounts.len()),
                Err(e) => eprintln!("Warning: Could not fetch accounts: {}", e),
            }
        }
        Err(e) => {
            eprintln!("OAuth2 login failed: {}", e);
            process::exit(1);
        }
    }
}

#[allow(deprecated)]
async fn run_legacy_login(args: Vec<String>) {
    let mut args = args.into_iter();
    let username = match args.next() {
        Some(u) => u,
        None => {
            eprintln!("Error: Missing username argument.");
            eprintln!("Usage: test_login <username> <password>");
            eprintln!("       test_login --oauth  (uses env vars)");
            process::exit(1);
        }
    };
    let password = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Error: Missing password argument.");
            eprintln!("Usage: test_login <username> <password>");
            process::exit(1);
        }
    };

    println!("Attempting legacy session login (demo environment)...");
    match TastyTrade::login_demo(&username, &password, false).await {
        Ok(_) => {
            println!("Login successful!");
        }
        Err(e) => {
            eprintln!("Login failed: {}", e);
            process::exit(1);
        }
    }
}
