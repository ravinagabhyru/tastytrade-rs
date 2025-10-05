use std::fmt::Display;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_with::serde_as;
use serde_with::VecSkipError;

#[derive(thiserror::Error, Debug, Deserialize)]
#[serde(untagged)]
pub enum TastyApiResponse<T> {
    Success(Response<T>),
    Error { error: ApiError },
}

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    pub data: T,
    pub context: Option<String>,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Pagination {
    pub per_page: usize,
    pub page_offset: usize,
    pub item_offset: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub current_item_count: usize,
    pub previous_link: Option<String>,
    pub next_link: Option<String>,
    pub paging_link_template: Option<String>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Items<T: DeserializeOwned> {
    // TODO: not this
    #[serde_as(as = "VecSkipError<_>")]
    pub items: Vec<T>,
}

pub struct Paginated<T> {
    pub items: Vec<T>,
    pub pagination: Pagination,
}

#[derive(thiserror::Error, Debug, Deserialize)]
pub struct ApiError {
    pub code: Option<String>,
    pub message: String,
    pub errors: Option<Vec<InnerApiError>>,
}

#[derive(Debug, Deserialize)]
pub struct InnerApiError {
    pub code: Option<String>,
    pub message: String,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error {:?}: {}", self.code, self.message)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TastyError {
    #[error("Tastyworks API error")]
    Api(#[from] ApiError),
    #[error("HTTP Error")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON Error")]
    Json(#[from] serde_json::Error),
    // #[error("DxFeed Error")]
    // DxFeed(#[from] crate::quote_streamer::DxFeedError),
    #[error("Websocket Error")]
    Websocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Transaction Query Error")]
    TransactionQuery(#[from] crate::api::transaction::TransactionQueryError),
    #[error("Unexpected response (status {status}): {body}")]
    UnexpectedResponse { status: u16, body: String },
}

pub type Result<T> = std::result::Result<T, TastyError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::FromTastyResponse;
    use serde_json::json;

    #[test]
    fn test_success_deserialization() {
        let json = json!({
            "data": {"test": "value"},
            "context": "test",
            "pagination": {
                "per-page": 10,
                "page-offset": 0,
                "item-offset": 0,
                "total-items": 100,
                "total-pages": 10,
                "current-item-count": 10,
                "previous-link": null,
                "next-link": "/next",
                "paging-link-template": "/template"
            }
        });

        let response: TastyApiResponse<serde_json::Value> = serde_json::from_value(json).unwrap();

        match response {
            TastyApiResponse::Success(resp) => {
                assert_eq!(resp.data["test"], "value");
                assert_eq!(resp.context, Some("test".to_string()));
                assert!(resp.pagination.is_some());
                let pagination = resp.pagination.unwrap();
                assert_eq!(pagination.per_page, 10);
                assert_eq!(pagination.total_items, 100);
                assert_eq!(pagination.next_link, Some("/next".to_string()));
            }
            _ => panic!("Expected Success variant"),
        }
    }

    #[test]
    fn test_error_deserialization_simple() {
        let json = json!({
            "error": {
                "code": "TEST_ERROR",
                "message": "Test error message"
            }
        });

        let response: TastyApiResponse<serde_json::Value> = serde_json::from_value(json).unwrap();

        match response {
            TastyApiResponse::Error { error } => {
                assert_eq!(error.code, Some("TEST_ERROR".to_string()));
                assert_eq!(error.message, "Test error message");
                assert!(error.errors.is_none());
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_error_deserialization_with_nested_errors() {
        let json = json!({
            "error": {
                "code": "VALIDATION_ERROR",
                "message": "Validation failed",
                "errors": [
                    {
                        "code": "FIELD_ERROR",
                        "message": "Field is required"
                    },
                    {
                        "code": null,
                        "message": "Another error"
                    }
                ]
            }
        });

        let response: TastyApiResponse<serde_json::Value> = serde_json::from_value(json).unwrap();

        match response {
            TastyApiResponse::Error { error } => {
                assert_eq!(error.code, Some("VALIDATION_ERROR".to_string()));
                assert_eq!(error.message, "Validation failed");
                assert!(error.errors.is_some());

                let errors = error.errors.unwrap();
                assert_eq!(errors.len(), 2);
                assert_eq!(errors[0].code, Some("FIELD_ERROR".to_string()));
                assert_eq!(errors[0].message, "Field is required");
                assert_eq!(errors[1].code, None);
                assert_eq!(errors[1].message, "Another error");
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_api_error_display() {
        let error = ApiError {
            code: Some("X".to_string()),
            message: "msg".to_string(),
            errors: None,
        };

        assert_eq!(format!("{}", error), "Error Some(\"X\"): msg");
    }

    #[test]
    fn test_api_error_display_no_code() {
        let error = ApiError {
            code: None,
            message: "msg".to_string(),
            errors: None,
        };

        assert_eq!(format!("{}", error), "Error None: msg");
    }

    #[test]
    fn test_from_tasty_response_for_t() {
        let response = Response {
            data: "test_data".to_string(),
            context: Some("test".to_string()),
            pagination: None,
        };

        let result = String::from_tasty(response);
        assert_eq!(result, "test_data");
    }

    #[test]
    fn test_from_tasty_response_for_paginated() {
        let items = Items {
            items: vec!["item1".to_string(), "item2".to_string()],
        };

        let pagination = Pagination {
            per_page: 10,
            page_offset: 0,
            item_offset: 0,
            total_items: 2,
            total_pages: 1,
            current_item_count: 2,
            previous_link: None,
            next_link: None,
            paging_link_template: None,
        };

        let response = Response {
            data: items,
            context: Some("test".to_string()),
            pagination: Some(pagination),
        };

        let result = Paginated::<String>::from_tasty(response);
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0], "item1");
        assert_eq!(result.items[1], "item2");
        assert_eq!(result.pagination.total_items, 2);
    }

    #[test]
    #[should_panic(expected = "unwrap")]
    fn test_from_tasty_response_for_paginated_missing_pagination() {
        let items = Items {
            items: vec!["item1".to_string()],
        };

        let response = Response {
            data: items,
            context: Some("test".to_string()),
            pagination: None, // Missing pagination should cause panic
        };

        let _result = Paginated::<String>::from_tasty(response);
    }

    #[test]
    fn test_items_deserialization_skip_errors() {
        // Test that VecSkipError works - items with parsing errors are skipped
        let json = json!({
            "items": [
                {"valid": true},
                "invalid_item",  // This should be skipped
                {"valid": false}
            ]
        });

        #[derive(serde::Deserialize, PartialEq, Debug)]
        struct TestItem {
            valid: bool,
        }

        let items: Items<TestItem> = serde_json::from_value(json).unwrap();
        assert_eq!(items.items.len(), 2); // Invalid item was skipped
        assert_eq!(items.items[0].valid, true);
        assert_eq!(items.items[1].valid, false);
    }
    #[test]
    fn test_success_without_context() {
        let json = json!({
            "data": {"test": "value"},
            "pagination": null
        });

        let response: TastyApiResponse<serde_json::Value> = serde_json::from_value(json).unwrap();

        match response {
            TastyApiResponse::Success(resp) => {
                assert!(resp.context.is_none());
                assert!(resp.pagination.is_none());
            }
            _ => panic!("Expected Success variant"),
        }
    }
}
