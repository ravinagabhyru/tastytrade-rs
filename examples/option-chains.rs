use rust_decimal::prelude::*;
use std::collections::HashMap;
use std::process;
use std::time::Duration;
use tastytrade_rs::api::quote_streaming::{GreeksData, QuoteData, StreamerEventData};
use tastytrade_rs::TastyTrade;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let username = match args.next() {
        Some(u) => u,
        None => {
            eprintln!("Error: Missing username argument.");
            eprintln!("Usage: option-chains <username> <password> <demo/live> <symbol> [--strike-range min-max] [--max-dte days] [--with-streaming]");
            process::exit(1);
        }
    };
    let password = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Error: Missing password argument.");
            eprintln!("Usage: option-chains <username> <password> <demo/live> <symbol> [--strike-range min-max] [--max-dte days] [--with-streaming]");
            process::exit(1);
        }
    };
    let env = match args.next() {
        Some(e) => e,
        None => {
            eprintln!("Error: Missing environment argument.");
            eprintln!("Usage: option-chains <username> <password> <demo/live> <symbol> [--strike-range min-max] [--max-dte days] [--with-streaming]");
            process::exit(1);
        }
    };
    let symbol = match args.next() {
        Some(s) => s,
        None => {
            eprintln!("Error: Missing symbol argument.");
            eprintln!("Usage: option-chains <username> <password> <demo/live> <symbol> [--strike-range min-max] [--max-dte days] [--with-streaming]");
            process::exit(1);
        }
    };

    // Parse optional filters
    let mut strike_min: Option<Decimal> = None;
    let mut strike_max: Option<Decimal> = None;
    let mut max_dte: Option<u64> = None;
    let mut with_streaming: bool = false;

    let mut i = 0;
    let remaining_args: Vec<String> = args.collect();
    while i < remaining_args.len() {
        match remaining_args[i].as_str() {
            "--strike-range" => {
                if i + 1 < remaining_args.len() {
                    let range = &remaining_args[i + 1];
                    if let Some((min_str, max_str)) = range.split_once('-') {
                        strike_min = min_str.parse().ok();
                        strike_max = max_str.parse().ok();
                        if strike_min.is_none() || strike_max.is_none() {
                            eprintln!(
                                "Error: Invalid strike range format. Use: --strike-range 100-200"
                            );
                            process::exit(1);
                        }
                    } else {
                        eprintln!(
                            "Error: Invalid strike range format. Use: --strike-range 100-200"
                        );
                        process::exit(1);
                    }
                    i += 2;
                } else {
                    eprintln!("Error: --strike-range requires a value");
                    process::exit(1);
                }
            }
            "--max-dte" => {
                if i + 1 < remaining_args.len() {
                    max_dte = remaining_args[i + 1].parse().ok();
                    if max_dte.is_none() {
                        eprintln!("Error: Invalid max-dte value. Must be a number.");
                        process::exit(1);
                    }
                    i += 2;
                } else {
                    eprintln!("Error: --max-dte requires a value");
                    process::exit(1);
                }
            }
            "--with-streaming" => {
                with_streaming = true;
                i += 1;
            }
            _ => {
                eprintln!("Error: Unknown argument: {}", remaining_args[i]);
                process::exit(1);
            }
        }
    }

    let live = env == "live";

    // Login
    let login_result = if live {
        TastyTrade::login(&username, &password, false).await
    } else {
        TastyTrade::login_demo(&username, &password, false).await
    };

    let tasty = match login_result {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Login failed: {}", e);
            process::exit(1);
        }
    };

    println!("=== {} Options Chain ===", symbol);
    if let (Some(min), Some(max)) = (strike_min, strike_max) {
        println!("Strike Range: ${} - ${}", min, max);
    }
    if let Some(dte) = max_dte {
        println!("Max DTE: {} days", dte);
    }
    if with_streaming {
        println!("Streaming: Enabled (will fetch live quotes and Greeks)");
    }
    println!();

    // Initialize streaming data storage
    let mut streaming_data: HashMap<String, (Option<QuoteData>, Option<GreeksData>)> =
        HashMap::new();

    // Fetch nested option chain
    let chain = match tasty.nested_option_chain_for(&symbol).await {
        Ok(chain) => chain,
        Err(e) => {
            eprintln!("Failed to fetch option chain for {}: {}", symbol, e);
            process::exit(1);
        }
    };

    // Collect streaming data FIRST if requested
    if with_streaming {
        println!("\n=== Fetching Live Market Data ===");

        // Collect all option symbols that pass our filters and get their streamer symbols
        let mut regular_symbols = Vec::new();
        for expiration in &chain.expirations {
            // Apply DTE filter
            if let Some(max_dte) = max_dte {
                if expiration.days_to_expiration > max_dte {
                    continue;
                }
            }

            for strike in &expiration.strikes {
                // Apply strike range filter
                if let (Some(min), Some(max)) = (strike_min, strike_max) {
                    if strike.strike_price < min || strike.strike_price > max {
                        continue;
                    }
                }

                regular_symbols.push(strike.call.0.clone());
                regular_symbols.push(strike.put.0.clone());
            }
        }

        // Convert regular symbols to streamer symbols
        let mut option_symbols = Vec::new();
        let mut symbol_mapping = HashMap::new(); // streamer_symbol -> regular_symbol

        println!(
            "Converting {} option symbols to DxLink format...",
            regular_symbols.len()
        );
        for (i, symbol) in regular_symbols.iter().enumerate() {
            if i % 10 == 0 && i > 0 {
                println!("Processed {}/{} symbols...", i, regular_symbols.len());
            }

            match tasty.get_option_info(symbol).await {
                Ok(option_info) => {
                    option_symbols.push(option_info.streamer_symbol.0.clone());
                    symbol_mapping.insert(option_info.streamer_symbol.0.clone(), symbol.clone());
                }
                Err(e) => {
                    println!(
                        "Warning: Failed to get streamer symbol for {}: {}",
                        symbol, e
                    );
                }
            }
        }

        println!(
            "Successfully converted {} symbols to DxLink format",
            option_symbols.len()
        );

        if !option_symbols.is_empty() {
            println!("Connecting to streaming service...");
            match tasty.create_dxlink_quote_streamer().await {
                Ok(mut streamer) => {
                    println!("Creating streaming channel...");
                    match streamer.create_default_channel().await {
                        Ok(channel_id) => {
                            println!("Subscribing to {} option symbols...", option_symbols.len());

                            // Subscribe to both quotes and Greeks
                            if let Err(e) =
                                streamer.subscribe_quotes(channel_id, &option_symbols).await
                            {
                                println!("Warning: Failed to subscribe to quotes: {}", e);
                            }

                            if let Err(e) =
                                streamer.subscribe_greeks(channel_id, &option_symbols).await
                            {
                                println!("Warning: Failed to subscribe to Greeks: {}", e);
                            }

                            println!("Collecting market data for 30 seconds...");
                            let timeout = tokio::time::sleep(Duration::from_secs(30));
                            tokio::pin!(timeout);

                            loop {
                                tokio::select! {
                                    _ = &mut timeout => {
                                        println!("Collected streaming data for {} symbols", streaming_data.len());
                                        break;
                                    }
                                    event_result = streamer.receive_event() => {
                                        match event_result {
                                            Ok(Some((_channel, event))) => {
                                                // Map streamer symbol back to regular symbol for display
                                                let streamer_symbol = match &event.data {
                                                    StreamerEventData::Quote(q) => &q.symbol,
                                                    StreamerEventData::Greeks(g) => &g.symbol,
                                                };

                                                if let Some(regular_symbol) = symbol_mapping.get(streamer_symbol) {
                                                    let entry = streaming_data.entry(regular_symbol.clone()).or_insert((None, None));

                                                    match event.data {
                                                        StreamerEventData::Quote(quote_data) => {
                                                            entry.0 = Some(quote_data);
                                                        }
                                                        StreamerEventData::Greeks(greeks_data) => {
                                                            entry.1 = Some(greeks_data);
                                                        }
                                                    }
                                                }
                                            }
                                            Ok(None) => continue,
                                            Err(e) => {
                                                println!("Error receiving streaming data: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => println!("Failed to create streaming channel: {}", e),
                    }
                }
                Err(e) => println!("Failed to create streaming connection: {}", e),
            }
        } else {
            println!("No options match the specified filters for streaming.");
        }
    }

    // Now display the option chain WITH the collected streaming data
    println!("\n=== Options Chain Display ===");
    println!(
        "Underlying: {} | Root: {} | Type: {}",
        chain.underlying_symbol.0, chain.root_symbol.0, chain.option_chain_type
    );
    println!("Shares per Contract: {}", chain.shares_per_contract);
    println!();

    for expiration in &chain.expirations {
        // Apply DTE filter
        if let Some(max_dte) = max_dte {
            if expiration.days_to_expiration > max_dte {
                continue;
            }
        }

        println!(
            "┌─ Expiration: {} ({} days) - {} Settlement ({})",
            expiration.expiration_date,
            expiration.days_to_expiration,
            expiration.expiration_type,
            expiration.settlement_type
        );

        for strike in &expiration.strikes {
            // Apply strike range filter
            if let (Some(min), Some(max)) = (strike_min, strike_max) {
                if strike.strike_price < min || strike.strike_price > max {
                    continue;
                }
            }

            println!("├─ Strike: ${}", strike.strike_price);

            // Display call with streaming data if available
            println!("│  ├─ Call: {}", strike.call.0);
            if let Some((quote_opt, greeks_opt)) = streaming_data.get(&strike.call.0) {
                if let Some(quote) = quote_opt {
                    if let (Some(bid), Some(ask)) = (quote.bid_price, quote.ask_price) {
                        println!("│  │   ├─ Bid: ${:.2} | Ask: ${:.2}", bid, ask);
                    }
                }
                if let Some(greeks) = greeks_opt {
                    print!("│  │   └─ Greeks: ");
                    let mut parts = Vec::new();
                    if let Some(delta) = greeks.delta {
                        parts.push(format!("δ:{:.2}", delta));
                    }
                    if let Some(gamma) = greeks.gamma {
                        parts.push(format!("γ:{:.3}", gamma));
                    }
                    if let Some(theta) = greeks.theta {
                        parts.push(format!("θ:{:.2}", theta));
                    }
                    if let Some(vega) = greeks.vega {
                        parts.push(format!("ν:{:.2}", vega));
                    }
                    if let Some(rho) = greeks.rho {
                        parts.push(format!("ρ:{:.2}", rho));
                    }
                    if let Some(iv) = greeks.volatility {
                        parts.push(format!("IV:{:.1}%", iv * 100.0));
                    }
                    if parts.is_empty() {
                        println!("No data");
                    } else {
                        println!("{}", parts.join(" "));
                    }
                }
            }

            // Display put with streaming data if available
            println!("│  └─ Put:  {}", strike.put.0);
            if let Some((quote_opt, greeks_opt)) = streaming_data.get(&strike.put.0) {
                if let Some(quote) = quote_opt {
                    if let (Some(bid), Some(ask)) = (quote.bid_price, quote.ask_price) {
                        println!("│      ├─ Bid: ${:.2} | Ask: ${:.2}", bid, ask);
                    }
                }
                if let Some(greeks) = greeks_opt {
                    print!("│      └─ Greeks: ");
                    let mut parts = Vec::new();
                    if let Some(delta) = greeks.delta {
                        parts.push(format!("δ:{:.2}", delta));
                    }
                    if let Some(gamma) = greeks.gamma {
                        parts.push(format!("γ:{:.3}", gamma));
                    }
                    if let Some(theta) = greeks.theta {
                        parts.push(format!("θ:{:.2}", theta));
                    }
                    if let Some(vega) = greeks.vega {
                        parts.push(format!("ν:{:.2}", vega));
                    }
                    if let Some(rho) = greeks.rho {
                        parts.push(format!("ρ:{:.2}", rho));
                    }
                    if let Some(iv) = greeks.volatility {
                        parts.push(format!("IV:{:.1}%", iv * 100.0));
                    }
                    if parts.is_empty() {
                        println!("No data");
                    } else {
                        println!("{}", parts.join(" "));
                    }
                }
            }
        }
    }

    // Display additional metadata from flat chain if needed
    if !with_streaming {
        println!("\n=== Additional Option Data (Metadata Only) ===");
        match tasty.option_chain_for(&symbol).await {
            Ok(flat_chain) => {
                if flat_chain.is_empty() {
                    println!("No additional option data available.");
                } else {
                    println!("Found {} option contracts with metadata (no Greeks/quotes - use --with-streaming for live data)", flat_chain.len());
                    if let Some(first_option) = flat_chain.first() {
                        println!("\nAvailable metadata fields:");
                        for key in first_option.extra.keys() {
                            println!("  - {}", key);
                        }
                        println!("\nNote: Pricing data and Greeks are only available via streaming (--with-streaming flag)");
                    }
                }
            }
            Err(e) => {
                println!("Failed to fetch additional option data: {}", e);
            }
        }
    }

    println!("\n=== Options Chain Fetch Complete ===");
}
