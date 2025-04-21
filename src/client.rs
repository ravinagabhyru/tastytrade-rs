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

// use crate::api::base::TastyError;
use crate::api::login::LoginCredentials;
use crate::api::login::LoginResponse;

// use reqwest_inspect_json::InspectJson;

pub const BASE_URL: &str = "https://api.tastyworks.com";
pub const BASE_DEMO_URL: &str = "https://api.cert.tastyworks.com";

#[derive(Debug, Clone)]
pub struct TastyTrade {
    pub(crate) client: reqwest::Client,
    pub(crate) session_token: String,
    base_url: &'static str,
    pub(crate) demo: bool,
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
    pub async fn login(login: &str, password: &str, remember_me: bool) -> Result<Self> {
        let creds = Self::do_login_request(login, password, remember_me, BASE_URL).await?;
        let client = Self::create_client(&creds);

        Ok(Self {
            client,
            session_token: creds.session_token,
            base_url: "https://api.tastyworks.com",
            demo: false,
        })
    }

    pub async fn login_demo(login: &str, password: &str, remember_me: bool) -> Result<Self> {
        let creds = Self::do_login_request(login, password, remember_me, BASE_DEMO_URL).await?;
        let client = Self::create_client(&creds);

        Ok(Self {
            client,
            session_token: creds.session_token,
            base_url: "https://api.cert.tastyworks.com",
            demo: true,
        })
    }

    fn create_client(creds: &LoginResponse) -> reqwest::Client {
        let mut headers = HeaderMap::new();

        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&creds.session_token).unwrap(),
        );
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str("application/json").unwrap(),
        );
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_str("tastytrade-rs").unwrap(),
        );

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

    pub async fn get_with_query<T, R, U>(&self, url: U, query: &[(&str, &str)]) -> Result<R>
    where
        T: DeserializeOwned,
        R: FromTastyResponse<T>,
        U: AsRef<str>,
    {
        let url = format!("{}{}", self.base_url, url.as_ref());

        let result = self
            .client
            .get(url)
            .query(query)
            .send()
            .await?
            // .inspect_json::<TastyApiResponse<T>, TastyError>(move |text| {
            //     println!("{:?}", std::any::type_name::<T>());
            //     println!("{text}");
            // })
            .json::<TastyApiResponse<T>>()
            .await?;

        match result {
            TastyApiResponse::Success(s) => Ok(R::from_tasty(s)),
            TastyApiResponse::Error { error } => Err(error.into()),
        }
    }

    pub async fn get<T: DeserializeOwned, U: AsRef<str>>(&self, url: U) -> Result<T> {
        // Special case for debugging quote-token API calls
        if url.as_ref() == "/api-quote-tokens" {
            let url_full = format!("{}{}", self.base_url, url.as_ref());
            println!("DEBUG: Fetching from URL: {}", url_full);
            
            let response = match self.client.get(&url_full).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    println!("DEBUG: Request error: {}", e);
                    return Err(e.into());
                }
            };
            
            println!("DEBUG: Response status: {}", response.status());
            
            // Get response body as text
            let body = match response.text().await {
                Ok(text) => {
                    println!("DEBUG: Raw response body: {}", text);
                    text
                },
                Err(e) => {
                    println!("DEBUG: Failed to get response text: {}", e);
                    return Err(e.into());
                }
            };
            
            // Try using a different direct approach with serde_json::Value first
            let json_value: serde_json::Value = match serde_json::from_str(&body) {
                Ok(val) => val,
                Err(e) => {
                    println!("DEBUG: JSON parse error: {}", e);
                    return Err(e.into());
                }
            };
            
            // Now try to deserialize from the parsed Value
            match serde_json::from_value::<T>(json_value) {
                Ok(data) => return Ok(data),
                Err(e) => {
                    println!("DEBUG: Final deserialization error: {}", e);
                    return Err(e.into());
                }
            }
        }
        
        // Normal case
        self.get_with_query(url, &[]).await
    }

    pub async fn post<R, P, U>(&self, url: U, payload: P) -> Result<R>
    where
        R: DeserializeOwned,
        P: Serialize,
        U: AsRef<str>,
    {
        let url = format!("{}{}", self.base_url, url.as_ref());
        let result = self
            .client
            .post(url)
            .body(serde_json::to_string(&payload).unwrap())
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
        let url = format!("{}{}", self.base_url, url.as_ref());
        let result = self
            .client
            .delete(url)
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
