use dxlink_rs::feed::FeedContract;
use std::process;
use std::time::Duration;
use tastytrade_rs::api::oauth2::OAuth2Config;
use tastytrade_rs::api::quote_streaming::StreamerEventData;
use tastytrade_rs::TastyTrade;

use tokio::time;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    println!("Starting quote-streaming example with multi-channel support");

    // Initialize tracing subscriber
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("Logger initialized. Starting quote-streaming example");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let live = args.first().map(|s| s.as_str()) == Some("live");

    let client_id = std::env::var("TT_OAUTH_CLIENT_ID").unwrap_or_else(|_| {
        error!("Error: TT_OAUTH_CLIENT_ID environment variable not set");
        error!("Usage: quote-streaming [live]");
        error!("Required env vars: TT_OAUTH_CLIENT_ID, TT_OAUTH_CLIENT_SECRET, TT_OAUTH_REFRESH_TOKEN");
        process::exit(1);
    });
    let client_secret = std::env::var("TT_OAUTH_CLIENT_SECRET").unwrap_or_else(|_| {
        error!("Error: TT_OAUTH_CLIENT_SECRET environment variable not set");
        process::exit(1);
    });
    let redirect_uri = std::env::var("TT_OAUTH_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost".to_string());
    let refresh_token = std::env::var("TT_OAUTH_REFRESH_TOKEN").unwrap_or_else(|_| {
        error!("Error: TT_OAUTH_REFRESH_TOKEN environment variable not set");
        process::exit(1);
    });

    let config = OAuth2Config {
        client_id,
        client_secret,
        redirect_uri,
        scopes: vec!["read".to_string()],
    };

    let env_name = if live { "production" } else { "demo" };
    info!("Attempting login ({} environment)...", env_name);

    let tasty = match TastyTrade::from_refresh_token(config, &refresh_token, !live).await {
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
        }
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
    if let Err(e) = streamer
        .subscribe_quotes(quote_channel_id, quote_symbols)
        .await
    {
        error!("Failed to subscribe to quotes: {}", e);
        process::exit(1);
    }
    info!("Subscribed to quotes for symbols: {:?}", quote_symbols);

    // Subscribe to Summary events (includes open interest for options)
    let summary_symbols = &["SPY   251213C00650000", "SPY   251213P00650000"];
    if let Err(e) = streamer
        .subscribe_summary(quote_channel_id, summary_symbols)
        .await
    {
        error!("Failed to subscribe to summary events: {}", e);
        process::exit(1);
    }
    info!("Subscribed to summary events for symbols: {:?}", summary_symbols);

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
            Ok(Some((channel_id, ev))) => match &ev.data {
                StreamerEventData::Quote(quote_data) => {
                    info!(
                        "[Channel {}] QUOTE {}: Bid={:.2}, Ask={:.2} (Size: {}x{}) Time: {:?}",
                        channel_id,
                        quote_data.symbol,
                        quote_data.bid_price.unwrap_or(f64::NAN),
                        quote_data.ask_price.unwrap_or(f64::NAN),
                        quote_data.bid_size.unwrap_or(f64::NAN),
                        quote_data.ask_size.unwrap_or(f64::NAN),
                        quote_data.event_time
                    );
                }
                StreamerEventData::Greeks(greeks_data) => {
                    info!(
                        "[Channel {}] GREEKS {}: δ:{:.3}, γ:{:.4}, θ:{:.3}, ν:{:.3}, ρ:{:.3}, IV:{:.1}%",
                        channel_id,
                        greeks_data.symbol,
                        greeks_data.delta.unwrap_or(f64::NAN),
                        greeks_data.gamma.unwrap_or(f64::NAN),
                        greeks_data.theta.unwrap_or(f64::NAN),
                        greeks_data.vega.unwrap_or(f64::NAN),
                        greeks_data.rho.unwrap_or(f64::NAN),
                        greeks_data.volatility.map(|v| v * 100.0).unwrap_or(f64::NAN)
                    );
                }
                StreamerEventData::Summary(summary_data) => {
                    info!(
                        "[Channel {}] SUMMARY {}: Open={:.2}, High={:.2}, Low={:.2}, Close={:.2}, PrevClose={:.2}, OpenInt={:.0}",
                        channel_id,
                        summary_data.symbol,
                        summary_data.day_open_price.unwrap_or(f64::NAN),
                        summary_data.day_high_price.unwrap_or(f64::NAN),
                        summary_data.day_low_price.unwrap_or(f64::NAN),
                        summary_data.day_close_price.unwrap_or(f64::NAN),
                        summary_data.prev_day_close_price.unwrap_or(f64::NAN),
                        summary_data.open_interest.unwrap_or(f64::NAN)
                    );
                }
            },
            Ok(None) => {
                // No events available, sleep briefly
                time::sleep(Duration::from_millis(100)).await;
            }
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
