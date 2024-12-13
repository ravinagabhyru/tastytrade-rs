use tastytrade_rs::dxfeed;
use tastytrade_rs::TastyTrade;
use rpassword::read_password;
use std::process;
use dxfeed::EventData::Quote;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let username = match args.next() {
        Some(u) => u,
        None => {
            eprintln!("Username not provided");
            process::exit(1);
        }
    };

    println!("Enter password: ");
    let password = match read_password() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to read password: {}", e);
            process::exit(1);
        }
    };

    let tasty = match TastyTrade::login(&username, &password, false).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Login failed: {}", e);
            process::exit(1);
        }
    };

    let streamer = match tasty.create_quote_streamer().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create quote streamer: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = streamer.with_subscriber(&["SPX"]).await {
        eprintln!("Failed to subscribe to quotes: {}", e);
        process::exit(1);
    }

    while let Ok(ev) = streamer.get_event().await {
        if let Quote(data) = ev.data {
            println!("{}: {}/{}", ev.sym, data.bid_price, data.ask_price);
        }
    }
}