use std::process;
use tastytrade_rs::TastyTrade;

use tracing::{error, info};

#[tokio::main]
async fn main() {
    println!("Starting quote-streaming example");

    // Initialize tracing subscriber
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    info!("Logger initialized. Starting quote-streaming example");

    let mut args = std::env::args().skip(1);
    let username = match args.next() {
        Some(u) => u,
        None => {
            error!("Error: Missing username argument.");
            error!("Usage: quote-streaming <username> <password>");
            process::exit(1);
        }
    };
    let password = match args.next() {
        Some(p) => p,
        None => {
            error!("Error: Missing password argument.");
            error!("Usage: quote-streaming <username> <password>");
            process::exit(1);
        }
    };

    let live = match args.next() {
        Some(p) => {
            if p == "live" {
                true
            } else {
                false
            }
        }
        None => false,
    };

    info!("Attempting to login with username: {}", username);
    let login_result = if live {
        TastyTrade::login(&username, &password, false).await
    } else {
        TastyTrade::login_demo(&username, &password, false).await
    };

    let tasty = match login_result {
        Ok(t) => {
            info!("Login successful");
            t
        }
        Err(e) => {
            error!("Login failed: {}", e);
            process::exit(1);
        }
    };

    info!("Creating dxLink quote streamer...");
    let mut streamer = match tasty.create_dxlink_quote_streamer().await {
        Ok(s) => {
            info!("Successfully created dxLink quote streamer");
            s
        }
        Err(e) => {
            error!("Failed to create dxLink quote streamer: {}", e);
            error!("Error details: {:?}", e);
            process::exit(1);
        }
    };

    // Initialize the receiver
    streamer.initialize_receiver();

    let symbols_to_subscribe = &["AAPL"];
    if let Err(e) = streamer.subscribe_quotes(symbols_to_subscribe).await {
        error!("Failed to subscribe to quotes: {}", e);
        process::exit(1);
    }
    info!("Subscribed to: {:?}", symbols_to_subscribe);

    loop {
        match streamer.receive_event().await {
            Ok(Some(ev)) => {
                if ev.event_type == "Quote" {
                    info!(
                        "{}: Bid={:.2}, Ask={:.2} (Size: {}x{}) Time: {:?}",
                        ev.data.symbol,
                        ev.data.bid_price.unwrap_or(f64::NAN),
                        ev.data.ask_price.unwrap_or(f64::NAN),
                        ev.data.bid_size.unwrap_or(f64::NAN),
                        ev.data.ask_size.unwrap_or(f64::NAN),
                        ev.data.event_time
                    );
                }
            }
            Ok(None) => {
                // Ignored event type, continue
            }
            Err(e) => {
                error!("Error receiving quote event: {}", e);
                break;
            }
        }
    }
}
