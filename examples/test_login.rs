use std::process;
use tastytrade_rs::TastyTrade;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let username = match args.next() {
        Some(u) => u,
        None => {
            eprintln!("Error: Missing username argument.");
            eprintln!("Usage: test_login <username> <password>");
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

    // Always use demo login for this test
    let login_result = TastyTrade::login_demo(&username, &password, false).await;

    match login_result {
        Ok(_) => {
            println!("Login successful!");
        }
        Err(e) => {
            eprintln!("Login failed: {}", e);
            process::exit(1);
        }
    }
}
