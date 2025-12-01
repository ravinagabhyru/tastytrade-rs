use reqwest::header;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::ClientBuilder;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::api::base::Items;
use crate::api::base::Paginated;
use crate::api::base::Response;
use crate::api::base::Result;
use crate::api::base::TastyApiResponse;
use crate::api::base::TastyError;
use crate::api::auth::AuthState;
use crate::api::oauth2::{OAuth2AuthRequest, OAuth2Config, OAuth2Token, OAuth2TokenResponse};
use tokio::sync::{Mutex, RwLock};
use chrono::Utc;
use url::Url;

pub const BASE_URL: &str = "https://api.tastyworks.com";
pub const BASE_DEMO_URL: &str = "https://api.cert.tastyworks.com";

pub struct TastyTrade {
    pub(crate) client: reqwest::Client,
    pub(crate) auth_state: RwLock<AuthState>,
    base_url: &'static str,
    pub(crate) demo: bool,
    refresh_lock: Mutex<()>,
}

pub trait FromTastyResponse<T: DeserializeOwned> {
    fn from_tasty(resp: Response<T>) -> Self;
}

impl<T: DeserializeOwned> FromTastyResponse<T> for T {
    fn from_tasty(resp: Response<T>) -> Self {
        resp.data
    }
}

impl<T: DeserializeOwned> FromTastyResponse<Items<T>> for Paginated<T> {
    fn from_tasty(resp: Response<Items<T>>) -> Self {
        Paginated {
            items: resp.data.items,
            pagination: resp.pagination.unwrap(),
        }
    }
}

impl TastyTrade {
    /// Create a client using a refresh token (personal grant flow)
    ///
    /// This is the simplest way to authenticate for personal applications.
    /// Generate a refresh token at my.tastytrade.com -> API -> OAuth Applications.
    pub async fn from_refresh_token(
        config: OAuth2Config,
        refresh_token: &str,
        demo: bool,
    ) -> Result<Self> {
        let base = if demo { BASE_DEMO_URL } else { BASE_URL };
        let token = Self::do_refresh_token(&config, refresh_token, base).await?;
        Self::create_client(config, token, demo)
    }

    /// Create a client from a saved token (auto-refreshes if expired)
    ///
    /// Use this to restore a session from a previously saved OAuth2Token.
    pub async fn from_token(config: OAuth2Config, token: OAuth2Token, demo: bool) -> Result<Self> {
        if token.is_expired() {
            Self::from_refresh_token(config, &token.refresh_token, demo).await
        } else {
            Self::create_client(config, token, demo)
        }
    }

    /// Create a client by exchanging an authorization code for tokens
    ///
    /// Use this after the user completes the browser-based authorization flow.
    pub async fn from_auth_code(config: OAuth2Config, code: &str, demo: bool) -> Result<Self> {
        let base = if demo { BASE_DEMO_URL } else { BASE_URL };
        let token = Self::exchange_code_for_token(&config, code, base).await?;
        Self::create_client(config, token, demo)
    }

    /// Build the authorization URL for browser-based code flow
    ///
    /// Direct users to this URL to authorize your application.
    pub fn authorize_url(config: &OAuth2Config, state: Option<&str>, demo: bool) -> String {
        let host = if demo {
            "https://cert-my.staging-tasty.works"
        } else {
            "https://my.tastytrade.com"
        };
        let mut url = Url::parse(&format!("{}/auth.html", host)).expect("valid authorize URL base");
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("client_id", &config.client_id);
            qp.append_pair("redirect_uri", &config.redirect_uri);
            qp.append_pair("response_type", "code");
            if !config.scopes.is_empty() {
                let scopes = config.scopes.join(" ");
                qp.append_pair("scope", &scopes);
            }
            if let Some(s) = state {
                qp.append_pair("state", s);
            }
        }
        url.into()
    }

    /// Get the current OAuth2 token for saving/persistence
    pub async fn get_token(&self) -> OAuth2Token {
        let guard = self.auth_state.read().await;
        OAuth2Token {
            access_token: guard.access_token.clone(),
            refresh_token: guard.refresh_token.clone().unwrap_or_default(),
            token_type: "Bearer".to_string(),
            expires_in: guard.expires_at
                .map(|exp| (exp - Utc::now()).num_seconds().max(0))
                .unwrap_or(3600),
            obtained_at: guard.expires_at
                .map(|exp| exp - chrono::Duration::seconds(3600))
                .unwrap_or_else(Utc::now),
            id_token: None,
        }
    }

    fn create_client(config: OAuth2Config, token: OAuth2Token, demo: bool) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(header::USER_AGENT, HeaderValue::from_static("tastytrade-rs"));
        let client = ClientBuilder::new().default_headers(headers).build()?;

        let expires_at = token.expires_at();
        Ok(Self {
            client,
            auth_state: RwLock::new(AuthState {
                access_token: token.access_token,
                refresh_token: Some(token.refresh_token),
                expires_at: Some(expires_at),
                config,
            }),
            base_url: if demo { BASE_DEMO_URL } else { BASE_URL },
            demo,
            refresh_lock: Mutex::new(()),
        })
    }

    async fn do_refresh_token(
        config: &OAuth2Config,
        refresh_token: &str,
        base_url: &str,
    ) -> Result<OAuth2Token> {
        let client = reqwest::Client::new();
        let body = OAuth2AuthRequest {
            grant_type: "refresh_token".to_string(),
            code: None,
            refresh_token: Some(refresh_token.to_string()),
            client_id: None,
            client_secret: config.client_secret.clone(),
            redirect_uri: None,
        };

        let resp = client
            .post(format!("{}/oauth/token", base_url))
            .header("Accept", "application/json")
            .header("Accept-Version", "20251101")
            .header("User-Agent", "tastytrade-rs/0.6.0")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(TastyError::UnexpectedResponse {
                status: status.as_u16(),
                body: text,
            });
        }

        let token_resp: OAuth2TokenResponse = serde_json::from_str(&text)?;
        // Pass original refresh_token as fallback since refresh responses may omit it
        Ok(OAuth2Token::from_response(token_resp, Some(refresh_token)))
    }

    async fn exchange_code_for_token(
        config: &OAuth2Config,
        code: &str,
        base_url: &str,
    ) -> Result<OAuth2Token> {
        let client = reqwest::Client::new();
        let body = OAuth2AuthRequest {
            grant_type: "authorization_code".to_string(),
            code: Some(code.to_string()),
            refresh_token: None,
            client_id: Some(config.client_id.clone()),
            client_secret: config.client_secret.clone(),
            redirect_uri: Some(config.redirect_uri.clone()),
        };

        let resp = client
            .post(format!("{}/oauth/token", base_url))
            .header("Accept", "application/json")
            .header("Accept-Version", "20251101")
            .header("User-Agent", "tastytrade-rs/0.6.0")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(TastyError::UnexpectedResponse {
                status: status.as_u16(),
                body: text,
            });
        }

        let token_resp: OAuth2TokenResponse = serde_json::from_str(&text)?;
        // Authorization code exchange must return a refresh_token
        Ok(OAuth2Token::from_response(token_resp, None))
    }

    /// Ensure the access token is valid; refresh if needed.
    async fn ensure_valid_token(&self) -> Result<()> {
        // Fast path without lock
        let needs_refresh = {
            let guard = self.auth_state.read().await;
            guard.needs_refresh()
        };
        if !needs_refresh {
            return Ok(());
        }

        // Serialize refresh under lock
        let _lock = self.refresh_lock.lock().await;
        // Recheck after acquiring the lock
        let maybe_refresh = {
            let guard = self.auth_state.read().await;
            if guard.needs_refresh() {
                Some((guard.config.clone(), guard.refresh_token.clone()))
            } else {
                None
            }
        };

        if let Some((config, Some(refresh_token))) = maybe_refresh {
            let base = if self.demo { BASE_DEMO_URL } else { BASE_URL };
            tracing::info!("Refreshing access token");
            let new_token = Self::do_refresh_token(&config, &refresh_token, base).await?;
            let expires_at = new_token.expires_at();
            let mut guard = self.auth_state.write().await;
            guard.access_token = new_token.access_token;
            guard.refresh_token = Some(new_token.refresh_token);
            guard.expires_at = Some(expires_at);
            tracing::info!("Access token refreshed");
        }
        Ok(())
    }

    pub async fn get_with_query<T, R, U>(&self, url: U, query: &[(&str, &str)]) -> Result<R>
    where
        T: DeserializeOwned,
        R: FromTastyResponse<T>,
        U: AsRef<str>,
    {
        self.ensure_valid_token().await?;
        let url = format!("{}{}", self.base_url, url.as_ref());

        let mut req = self.client.get(&url).query(query);
        let auth_header = { self.auth_state.read().await.auth_header() };
        req = req.header(header::AUTHORIZATION, auth_header);

        let response = req.send().await?;
        let status = response.status();
        let text = response.text().await?;

        tracing::debug!(
            "tastytrade GET {} status={} body={}",
            url,
            status.as_u16(),
            text
        );

        let result: TastyApiResponse<T> = match serde_json::from_str(&text) {
            Ok(parsed) => parsed,
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "failed to parse response for {} (status {}): {}",
                    url,
                    status,
                    text
                );
                return Err(TastyError::UnexpectedResponse {
                    status: status.as_u16(),
                    body: text,
                });
            }
        };

        if !status.is_success() {
            tracing::warn!(
                status = %status,
                url = %url,
                body = %text,
                "received non-success HTTP status from tastytrade"
            );
        }

        match result {
            TastyApiResponse::Success(s) => Ok(R::from_tasty(s)),
            TastyApiResponse::Error { error } => {
                tracing::error!(
                    code = ?error.code,
                    message = %error.message,
                    status = %status,
                    body = %text,
                    "tastytrade API returned error"
                );
                Err(error.into())
            }
        }
    }

    pub async fn get<T: DeserializeOwned, U: AsRef<str>>(&self, url: U) -> Result<T> {
        let res = self.get_with_query(url, &[]).await;
        res
    }

    pub async fn post<R, P, U>(&self, url: U, payload: P) -> Result<R>
    where
        R: DeserializeOwned,
        P: Serialize,
        U: AsRef<str>,
    {
        self.ensure_valid_token().await?;
        let url = format!("{}{}", self.base_url, url.as_ref());
        let mut req = self.client.post(url).body(serde_json::to_string(&payload).unwrap());
        let auth_header = { self.auth_state.read().await.auth_header() };
        req = req.header(header::AUTHORIZATION, auth_header);

        let result = req
            .send()
            .await?
            //.inspect_json::<TastyApiResponse<R>, TastyError>(move |text| {
            //    println!("{text}");
            //})
            .json::<TastyApiResponse<R>>()
            .await?;

        match result {
            TastyApiResponse::Success(s) => Ok(s.data),
            TastyApiResponse::Error { error } => Err(error.into()),
        }
    }

    pub async fn delete<R, U>(&self, url: U) -> Result<R>
    where
        R: DeserializeOwned,
        U: AsRef<str>,
    {
        self.ensure_valid_token().await?;
        let url = format!("{}{}", self.base_url, url.as_ref());
        let mut req = self.client.delete(url);
        let auth_header = { self.auth_state.read().await.auth_header() };
        req = req.header(header::AUTHORIZATION, auth_header);

        let result = req
            .send()
            .await?
            // .inspect_json::<TastyApiResponse<R>, TastyError>(move |text| {
            //     println!("{text}");
            // })
            .json::<TastyApiResponse<R>>()
            .await?;

        match result {
            TastyApiResponse::Success(s) => Ok(s.data),
            TastyApiResponse::Error { error } => Err(error.into()),
        }
    }
}
