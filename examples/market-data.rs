use std::collections::HashMap;
use std::process;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use tastytrade_rs::api::option_chain::Strike;
use tastytrade_rs::{MarketDataItem, MarketDataRequest, TastyTrade};

fn fmt_price(value: &Option<Decimal>) -> String {
    match value.as_ref().and_then(|d| d.to_f64()) {
        Some(v) => format!("{v:.2}"),
        None => value
            .as_ref()
            .map(|d| d.to_string())
            .unwrap_or_else(|| "-".to_string()),
    }
}

fn fmt_greek(value: &Option<Decimal>) -> String {
    match value.as_ref().and_then(|d| d.to_f64()) {
        Some(v) => format!("{v:.4}"),
        None => "-".to_string(),
    }
}

fn fmt_percent(value: &Option<Decimal>) -> String {
    match value.as_ref().and_then(|d| d.to_f64()) {
        Some(v) => format!("{:.2}", v * 100.0),
        None => "-".to_string(),
    }
}

fn underlying_price(item: &MarketDataItem) -> Option<Decimal> {
    item.mark
        .as_ref()
        .or_else(|| item.last.as_ref())
        .or_else(|| item.mid.as_ref())
        .or_else(|| item.bid.as_ref())
        .or_else(|| item.ask.as_ref())
        .cloned()
}

fn select_strikes<'a>(
    strikes: &'a [Strike],
    center_price: Option<Decimal>,
    limit: usize,
) -> Vec<&'a Strike> {
    let mut sorted: Vec<&Strike> = strikes.iter().collect();

    if let Some(target) = center_price {
        sorted.sort_by(|a, b| {
            let diff_a = (a.strike_price - target).abs();
            let diff_b = (b.strike_price - target).abs();
            diff_a
                .cmp(&diff_b)
                .then_with(|| a.strike_price.cmp(&b.strike_price))
        });
    } else {
        sorted.sort_by(|a, b| a.strike_price.cmp(&b.strike_price));
    }

    let mut selected: Vec<&Strike> = sorted.into_iter().take(limit).collect();
    selected.sort_by(|a, b| a.strike_price.cmp(&b.strike_price));
    selected
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let username = match args.next() {
        Some(u) => u,
        None => {
            eprintln!("Error: Missing username argument.");
            eprintln!("Usage: market-data <username> <password> <demo/live> <symbol>");
            process::exit(1);
        }
    };
    let password = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Error: Missing password argument.");
            eprintln!("Usage: market-data <username> <password> <demo/live> <symbol>");
            process::exit(1);
        }
    };
    let env = match args.next() {
        Some(e) => e,
        None => {
            eprintln!("Error: Missing environment argument (demo/live).");
            eprintln!("Usage: market-data <username> <password> <demo/live> <symbol>");
            process::exit(1);
        }
    };
    let symbol = match args.next() {
        Some(s) => s,
        None => {
            eprintln!("Error: Missing symbol argument.");
            eprintln!("Usage: market-data <username> <password> <demo/live> <symbol>");
            process::exit(1);
        }
    };

    let live = env.eq_ignore_ascii_case("live");

    let login_result = if live {
        TastyTrade::login(&username, &password, false).await
    } else {
        TastyTrade::login_demo(&username, &password, false).await
    };

    let tasty = match login_result {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Login failed: {e}");
            process::exit(1);
        }
    };

    println!("Fetching option chain for {symbol}...");
    let chain = match tasty.nested_option_chain_for(&symbol).await {
        Ok(chain) => chain,
        Err(e) => {
            eprintln!("Failed to load option chain: {e}");
            process::exit(1);
        }
    };

    let next_expiration = match chain.expirations.iter().min_by(|a, b| {
        a.days_to_expiration
            .cmp(&b.days_to_expiration)
            .then_with(|| a.expiration_date.cmp(&b.expiration_date))
    }) {
        Some(expiration) => expiration,
        None => {
            eprintln!("No expirations found for {symbol}.");
            process::exit(1);
        }
    };

    println!(
        "Using expiration {} ({} days to expiration)",
        next_expiration.expiration_date, next_expiration.days_to_expiration
    );

    let mut equity_request = MarketDataRequest::new();
    equity_request.add_equity(symbol.clone());

    let equity_only = match tasty.fetch_market_data(&equity_request).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to fetch equity quote for {symbol}: {e}");
            process::exit(1);
        }
    };

    let current_price = equity_only
        .iter()
        .find(|item| item.symbol == symbol)
        .and_then(underlying_price);

    if let Some(price) = current_price.as_ref().and_then(|d| d.to_f64()) {
        println!("Last known price: {price:.2}");
    } else {
        println!("Last known price: unavailable");
    }

    let selected_strikes = select_strikes(&next_expiration.strikes, current_price.clone(), 5);

    if selected_strikes.is_empty() {
        eprintln!("Unable to select strikes for {symbol}.");
        process::exit(1);
    }

    println!(
        "Collecting market data for {} strike levels...",
        selected_strikes.len()
    );

    let mut request = MarketDataRequest::new();
    request.add_equity(symbol.clone());

    for strike in &selected_strikes {
        request.add_equity_option(strike.call.0.clone());
        request.add_equity_option(strike.put.0.clone());
    }

    let market_data = match tasty.fetch_market_data(&request).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to fetch market data: {e}");
            process::exit(1);
        }
    };

    let mut by_symbol: HashMap<String, MarketDataItem> = HashMap::new();
    for item in market_data.into_iter().chain(equity_only.into_iter()) {
        by_symbol.insert(item.symbol.clone(), item);
    }

    println!("\n=== Underlying Quote ===");
    if let Some(underlying) = by_symbol.get(&symbol) {
        println!(
            "{:<10} Bid {:>8} | Ask {:>8} | Mark {:>8} | Volume {:>10}",
            underlying.symbol,
            fmt_price(&underlying.bid),
            fmt_price(&underlying.ask),
            fmt_price(&underlying.mark),
            fmt_price(&underlying.volume),
        );
    } else {
        println!("No underlying quote returned.");
    }

    println!(
        "\n=== Option Quotes for {} ===",
        next_expiration.expiration_date
    );
    println!(
        "{:<10} {:<4} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Strike", "Type", "Bid", "Ask", "Mark", "IV%", "Delta", "Theta"
    );

    for strike in &selected_strikes {
        let strike_label = strike.strike_price.to_string();
        if let Some(call) = by_symbol.get(&strike.call.0) {
            println!(
                "{:<10} {:<4} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
                strike_label,
                "CALL",
                fmt_price(&call.bid),
                fmt_price(&call.ask),
                fmt_price(&call.mark),
                fmt_percent(&call.implied_volatility),
                fmt_greek(&call.delta),
                fmt_greek(&call.theta),
            );
        } else {
            println!(
                "{:<10} {:<4} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
                strike_label, "CALL", "-", "-", "-", "-", "-", "-"
            );
        }

        if let Some(put) = by_symbol.get(&strike.put.0) {
            println!(
                "{:<10} {:<4} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
                strike_label,
                "PUT",
                fmt_price(&put.bid),
                fmt_price(&put.ask),
                fmt_price(&put.mark),
                fmt_percent(&put.implied_volatility),
                fmt_greek(&put.delta),
                fmt_greek(&put.theta),
            );
        } else {
            println!(
                "{:<10} {:<4} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
                strike_label, "PUT", "-", "-", "-", "-", "-", "-"
            );
        }

        println!("{:-<80}", "");
    }

    println!("Done.");
}
