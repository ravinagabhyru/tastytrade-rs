use std::process;
use std::time::Duration;
use tastytrade_rs::TastyTrade;
use dxlink_rs::feed::FeedContract;

use tracing::{error, info};
use tokio::time;

#[tokio::main]
async fn main() {
    println!("Starting quote-streaming example with multi-channel support");

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
        Some(p) => p == "live",
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

    // Create two channels - one for quotes and one for trades
    info!("Creating quote channel");
    let quote_channel_id = match streamer.create_channel(FeedContract::Auto, None).await {
        Ok(id) => {
            info!("Successfully created quote channel with ID: {}", id);
            id
        },
        Err(e) => {
            error!("Failed to create quote channel: {}", e);
            process::exit(1);
        }
    };

    // info!("Creating trade channel");
    // let trade_channel_id = match streamer.create_channel(FeedContract::Auto, None).await {
    //     Ok(id) => {
    //         info!("Successfully created trade channel with ID: {}", id);
    //         id
    //     },
    //     Err(e) => {
    //         error!("Failed to create trade channel: {}", e);
    //         process::exit(1);
    //     }
    // };

    // Subscribe to quotes on channel 1
    let quote_symbols = &["AAPL", "MSFT", "SPX"];
    if let Err(e) = streamer.subscribe_quotes(quote_channel_id, quote_symbols).await {
        error!("Failed to subscribe to quotes: {}", e);
        process::exit(1);
    }
    info!("Subscribed to quotes for symbols: {:?}", quote_symbols);

    // Subscribe to trades on channel 2
    // let trade_symbols = &["AAPL", "GOOG"];
    // if let Err(e) = streamer.subscribe_trades(trade_channel_id, trade_symbols).await {
    //     error!("Failed to subscribe to trades: {}", e);
    //     process::exit(1);
    // }
    // info!("Subscribed to trades for symbols: {:?}", trade_symbols);

    // Process events from both channels
    info!("Starting to receive events from multiple channels");
    let mut running = true;
    while running {
        match streamer.receive_event().await {
            Ok(Some((channel_id, ev))) => {
                match ev.event_type.as_str() {
                    "Quote" => {
                        info!(
                            "[Channel {}] QUOTE {}: Bid={:.2}, Ask={:.2} (Size: {}x{}) Time: {:?}",
                            channel_id,
                            ev.data.symbol,
                            ev.data.bid_price.unwrap_or(f64::NAN),
                            ev.data.ask_price.unwrap_or(f64::NAN),
                            ev.data.bid_size.unwrap_or(f64::NAN),
                            ev.data.ask_size.unwrap_or(f64::NAN),
                            ev.data.event_time
                        );
                    },
                    "Trade" => {
                        info!(
                            "[Channel {}] TRADE {}: Price={:.2}, Size={} Time: {:?}",
                            channel_id,
                            ev.data.symbol,
                            ev.data.ask_price.unwrap_or(f64::NAN),
                            ev.data.ask_size.unwrap_or(f64::NAN),
                            ev.data.event_time
                        );
                    },
                    _ => {
                        info!("[Channel {}] Unknown event type: {}", channel_id, ev.event_type);
                    }
                }
            },
            Ok(None) => {
                // No events available, sleep briefly
                time::sleep(Duration::from_millis(100)).await;
            },
            Err(e) => {
                error!("Error receiving events: {}", e);
                running = false;
            }
        }
    }

    // Cleanup before exiting
    info!("Closing channels...");
    if let Err(e) = streamer.close_channel(quote_channel_id).await {
        error!("Failed to close quote channel: {}", e);
    }

    // if let Err(e) = streamer.close_channel(trade_channel_id).await {
    //     error!("Failed to close trade channel: {}", e);
    // }

    info!("Example complete.");
}
