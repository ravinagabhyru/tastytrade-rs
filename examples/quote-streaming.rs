use tastytrade_rs::TastyTrade;
use std::process;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let username = args.next().unwrap();
    let password = args.next().unwrap();

    let tasty = match TastyTrade::login(&username, &password, false).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Login failed: {}", e);
            process::exit(1);
        }
    };

    let mut streamer = match tasty.create_dxlink_quote_streamer().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create dxLink quote streamer: {}", e);
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
