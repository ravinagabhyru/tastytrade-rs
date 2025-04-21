use tastytrade_rs::TastyTrade;
use std::process;

#[tokio::main]
async fn main() {
    println!("Starting quote-streaming example");
    let mut args = std::env::args().skip(1);
    let username = match args.next() {
        Some(u) => u,
        None => {
            eprintln!("Error: Missing username argument.");
            eprintln!("Usage: quote-streaming <username> <password>");
            process::exit(1);
        }
    };
    let password = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Error: Missing password argument.");
            eprintln!("Usage: quote-streaming <username> <password>");
            process::exit(1);
        }
    };

    println!("Attempting to login with username: {}", username);
    let tasty = match TastyTrade::login_demo(&username, &password, false).await {
        Ok(t) => {
            println!("Login successful");
            t
        },
        Err(e) => {
            eprintln!("Login failed: {}", e);
            process::exit(1);
        }
    };

    println!("Creating dxLink quote streamer...");
    let mut streamer = match tasty.create_dxlink_quote_streamer().await {
        Ok(s) => {
            println!("Successfully created dxLink quote streamer");
            s
        },
        Err(e) => {
            eprintln!("Failed to create dxLink quote streamer: {}", e);
            eprintln!("Error details: {:?}", e);
            process::exit(1);
        }
    };

    // Initialize the receiver
    streamer.initialize_receiver();

    let symbols_to_subscribe = &["SPX"];
    if let Err(e) = streamer.subscribe_quotes(symbols_to_subscribe).await {
        eprintln!("Failed to subscribe to quotes: {}", e);
        process::exit(1);
    }
    println!("Subscribed to: {:?}", symbols_to_subscribe);

    loop {
        match streamer.receive_event().await {
            Ok(Some(ev)) => {
                if ev.event_type == "Quote" {
                    println!(
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
                eprintln!("Error receiving quote event: {}", e);
                break;
            }
        }
    }
}
