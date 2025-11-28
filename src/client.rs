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
use crate::api::login::LoginCredentials;
use crate::api::login::LoginResponse;
use crate::api::auth::AuthMode;
use crate::api::oauth2::{OAuth2AuthRequest, OAuth2Config, OAuth2Token, OAuth2TokenResponse};
use tokio::sync::{Mutex, RwLock};
use chrono::Utc;
use url::Url;

// use reqwest_inspect_json::InspectJson;

pub const BASE_URL: &str = "https://api.tastyworks.com";
pub const BASE_DEMO_URL: &str = "https://api.cert.tastyworks.com";

pub struct TastyTrade {
    pub(crate) client: reqwest::Client,
    pub(crate) auth_mode: RwLock<AuthMode>,
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
    #[deprecated(since = "0.5.0", note = "Use OAuth2 authentication methods instead")]
    pub async fn login(login: &str, password: &str, remember_me: bool) -> Result<Self> {
        let creds = Self::do_login_request(login, password, remember_me, BASE_URL).await?;
        let client = Self::create_client();

        Ok(Self {
            client,
            auth_mode: RwLock::new(AuthMode::Session { session_token: creds.session_token }),
            base_url: "https://api.tastyworks.com",
            demo: false,
            refresh_lock: Mutex::new(()),
        })
    }

    #[deprecated(since = "0.5.0", note = "Use OAuth2 authentication methods instead")]
    pub async fn login_demo(login: &str, password: &str, remember_me: bool) -> Result<Self> {
        let creds = Self::do_login_request(login, password, remember_me, BASE_DEMO_URL).await?;
        let client = Self::create_client();

        Ok(Self {
            client,
            auth_mode: RwLock::new(AuthMode::Session { session_token: creds.session_token }),
            base_url: "https://api.cert.tastyworks.com",
            demo: true,
            refresh_lock: Mutex::new(()),
        })
    }

    fn create_client() -> reqwest::Client {
        let mut headers = HeaderMap::new();

        headers.insert(header::CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());
        headers.insert(header::USER_AGENT, HeaderValue::from_str("tastytrade-rs").unwrap());

        ClientBuilder::new()
            .default_headers(headers)
            .build()
            .expect("Could not create client")
    }

    async fn do_login_request(
        login: &str,
        password: &str,
        remember_me: bool,
        base_url: &str,
    ) -> Result<LoginResponse> {
        let client = reqwest::Client::default();

        let resp = client
            .post(format!("{base_url}/sessions"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::USER_AGENT, "tastytrade-rs")
            .json(&LoginCredentials {
                login,
                password,
                remember_me,
            })
            .send()
            .await?;
        let json = resp
            // .inspect_json::<TastyApiResponse<LoginResponse>, TastyError>(|text| println!("{text}"))
            .json()
            .await?;
        let response = match json {
            TastyApiResponse::Success(s) => Ok(s),
            TastyApiResponse::Error { error } => Err(error),
        }?
        .data;

        Ok(response)
    }

    // ===== OAuth2 Support =====

    /// Create an OAuth2 client using a refresh token (personal grant)
    pub async fn oauth2_from_refresh_token(
        config: OAuth2Config,
        refresh_token: &str,
        demo: bool,
    ) -> Result<Self> {
        let base = if demo { BASE_DEMO_URL } else { BASE_URL };
        let token = Self::refresh_oauth2_token(&config, refresh_token, base).await?;
        Self::create_oauth2_client(config, token, demo)
    }

    /// Create an OAuth2 client from a saved token (auto-refresh if needed)
    pub async fn oauth2_from_token(config: OAuth2Config, token: OAuth2Token, demo: bool) -> Result<Self> {
        if token.is_expired() {
            Self::oauth2_from_refresh_token(config, &token.refresh_token, demo).await
        } else {
            Self::create_oauth2_client(config, token, demo)
        }
    }

    /// Exchange authorization code for tokens
    pub async fn oauth2_exchange_code(config: OAuth2Config, code: &str, demo: bool) -> Result<Self> {
        let base = if demo { BASE_DEMO_URL } else { BASE_URL };
        let token = Self::exchange_code_for_token(&config, code, base).await?;
        Self::create_oauth2_client(config, token, demo)
    }

    /// Build the authorization URL for browser-based code flow
    pub fn oauth2_authorize_url(config: &OAuth2Config, state: Option<&str>, demo: bool) -> String {
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

    /// Accessor for the current OAuth2 token (if applicable)
    pub async fn get_oauth2_token(&self) -> Option<OAuth2Token> {
        let guard = self.auth_mode.read().await;
        match &*guard {
            AuthMode::OAuth2 { access_token, refresh_token, expires_at, .. } => {
                Some(OAuth2Token {
                    access_token: access_token.clone(),
                    refresh_token: refresh_token.clone().unwrap_or_default(),
                    token_type: "Bearer".to_string(),
                    expires_in: expires_at
                        .map(|exp| (exp - Utc::now()).num_seconds().max(0))
                        .unwrap_or(3600),
                    obtained_at: expires_at.map(|exp| exp - chrono::Duration::seconds(3600)).unwrap_or_else(Utc::now),
                    id_token: None,
                })
            }
            _ => None,
        }
    }

    fn create_oauth2_client(config: OAuth2Config, token: OAuth2Token, demo: bool) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(header::USER_AGENT, HeaderValue::from_static("tastytrade-rs"));
        let client = ClientBuilder::new().default_headers(headers).build()?;

        let expires_at = token.expires_at();
        let refresh_token = token.refresh_token.clone();
        Ok(Self {
            client,
            auth_mode: RwLock::new(AuthMode::OAuth2 {
                access_token: token.access_token,
                refresh_token: Some(refresh_token),
                expires_at: Some(expires_at),
                config,
            }),
            base_url: if demo { BASE_DEMO_URL } else { BASE_URL },
            demo,
            refresh_lock: Mutex::new(()),
        })
    }

    async fn refresh_oauth2_token(
        config: &OAuth2Config,
        refresh_token: &str,
        base_url: &str,
    ) -> Result<OAuth2Token> {
        let client = reqwest::Client::new();
        let body = OAuth2AuthRequest {
            grant_type: "refresh_token".to_string(),
            code: None,
            refresh_token: Some(refresh_token.to_string()),
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri: None,
        };

        let resp = client
            .post(format!("{}/oauth/token", base_url))
            .json(&body)
            .send()
            .await?;
        let token_resp: OAuth2TokenResponse = resp.json().await?;
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
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri: Some(config.redirect_uri.clone()),
        };

        let resp = client
            .post(format!("{}/oauth/token", base_url))
            .json(&body)
            .send()
            .await?;
        let token_resp: OAuth2TokenResponse = resp.json().await?;
        // Authorization code exchange must return a refresh_token
        Ok(OAuth2Token::from_response(token_resp, None))
    }

    /// Ensure the OAuth2 token is valid; refresh if needed. No-op for Session auth.
    async fn ensure_valid_token(&self) -> Result<()> {
        // Fast path without lock
        let needs_refresh = {
            let guard = self.auth_mode.read().await;
            guard.needs_refresh()
        };
        if !needs_refresh {
            return Ok(());
        }

        // Serialize refresh under lock
        let _lock = self.refresh_lock.lock().await;
        // Recheck after acquiring the lock
        let maybe_refresh = {
            let guard = self.auth_mode.read().await;
            match &*guard {
                AuthMode::OAuth2 { config, refresh_token, .. } if guard.needs_refresh() => {
                    Some((config.clone(), refresh_token.clone()))
                }
                _ => None,
            }
        };

        if let Some((config, refresh_token)) = maybe_refresh {
            if let Some(refresh_token) = refresh_token {
                let base = if self.demo { BASE_DEMO_URL } else { BASE_URL };
                tracing::info!("Refreshing OAuth2 access token");
                let new_token = Self::refresh_oauth2_token(&config, &refresh_token, base).await?;
                let mut guard = self.auth_mode.write().await;
                *guard = AuthMode::OAuth2 {
                    access_token: new_token.access_token.clone(),
                    refresh_token: Some(new_token.refresh_token.clone()),
                    expires_at: Some(new_token.expires_at()),
                    config,
                };
                tracing::info!("OAuth2 access token refreshed");
            }
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
        // attach Authorization per request
        let auth_header = { self.auth_mode.read().await.auth_header() };
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
        // attach Authorization per request
        let auth_header = { self.auth_mode.read().await.auth_header() };
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
        // attach Authorization per request
        let auth_header = { self.auth_mode.read().await.auth_header() };
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
