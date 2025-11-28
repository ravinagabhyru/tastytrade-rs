use tastytrade_rs::{TastyTrade, api::oauth2::{OAuth2ClientBuilder, OAuth2Config}};

/// OAuth2 integration tests; all require manual setup and are ignored by default.

#[tokio::test]
#[ignore = "Requires valid OAuth2 client and refresh token in env"]
async fn personal_grant_refresh_flow() {
    let client_id = std::env::var("TT_OAUTH_CLIENT_ID").expect("TT_OAUTH_CLIENT_ID not set");
    let client_secret = std::env::var("TT_OAUTH_CLIENT_SECRET").expect("TT_OAUTH_CLIENT_SECRET not set");
    let redirect_uri = std::env::var("TT_OAUTH_REDIRECT_URI").unwrap_or_else(|_| "http://localhost".to_string());
    let refresh_token = std::env::var("TT_OAUTH_REFRESH_TOKEN").expect("TT_OAUTH_REFRESH_TOKEN not set");

    let config = OAuth2ClientBuilder::new()
        .client_id(client_id)
        .client_secret(client_secret)
        .redirect_uri(redirect_uri)
        .add_scope("read")
        .build()
        .unwrap();

    let tasty = TastyTrade::from_refresh_token(config, &refresh_token, true).await
        .expect("Failed to create client from refresh token");

    // Basic sanity: fetch accounts
    let _accounts = tasty.accounts().await.expect("Failed to fetch accounts");
}

#[tokio::test]
#[ignore = "Manual: run browser flow to get code then set TT_OAUTH_AUTH_CODE"]
async fn authorization_code_exchange_flow() {
    let client_id = std::env::var("TT_OAUTH_CLIENT_ID").expect("TT_OAUTH_CLIENT_ID not set");
    let client_secret = std::env::var("TT_OAUTH_CLIENT_SECRET").expect("TT_OAUTH_CLIENT_SECRET not set");
    let redirect_uri = std::env::var("TT_OAUTH_REDIRECT_URI").unwrap_or_else(|_| "http://localhost".to_string());
    let code = std::env::var("TT_OAUTH_AUTH_CODE").expect("TT_OAUTH_AUTH_CODE not set");

    let config = OAuth2Config { client_id, client_secret, redirect_uri, scopes: vec!["read".into()] };
    let tasty = TastyTrade::from_auth_code(config, &code, true).await
        .expect("Failed to exchange code for token");

    let _accounts = tasty.accounts().await.expect("Failed to fetch accounts");
}

