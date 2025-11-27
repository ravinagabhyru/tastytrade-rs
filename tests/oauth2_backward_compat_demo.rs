use tastytrade_rs::TastyTrade;

mod test_utils;
use test_utils::env::demo_credentials;

#[tokio::test]
#[ignore = "Requires demo credentials; verifies legacy session still works"]
async fn legacy_session_login_still_works() {
    let (username, password) = demo_credentials().expect("Demo credentials required");
    let tasty = TastyTrade::login_demo(&username, &password, false).await
        .expect("Demo login should still work");

    let accounts = tasty.accounts().await.expect("Fetch accounts failed");
    assert!(!accounts.is_empty());
}

