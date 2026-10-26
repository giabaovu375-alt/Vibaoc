use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

use crate::runtime::value::VbValue;

pub struct ApiResult {
    pub ok: bool,
    pub status: u16,
    pub data: VbValue,
    pub error: Option<String>,
}

impl ApiResult {
    fn failure(status: u16, error: impl Into<String>) -> Self {
        ApiResult {
            ok: false,
            status,
            data: VbValue::Null,
            error: Some(error.into()),
        }
    }
}

pub fn resolve_url(base_url: &str, endpoint: &str) -> String {
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        return endpoint.to_string();
    }
    let base = base_url.trim_end_matches('/');
    let path = endpoint.trim_start_matches('/');
    if base.is_empty() {
        format!("/{}", path)
    } else {
        format!("{}/{}", base, path)
    }
}

pub async fn call(
    base_url: &str,
    method: &str,
    endpoint: &str,
    data: Option<&VbValue>,
    headers: Option<&VbValue>,
) -> ApiResult {
    let url = resolve_url(base_url, endpoint);
    let method_upper = method.to_uppercase();

    let opts = RequestInit::new();
    opts.set_method(&method_upper);

    if let Some(body_value) = data {
        if method_upper != "GET" && method_upper != "DELETE" {
            let body_json = body_value.to_json_string();
            opts.set_body(&JsValue::from_str(&body_json));
        }
    }

    let request = match Request::new_with_str_and_init(&url, &opts) {
        Ok(r) => r,
        Err(_) => return ApiResult::failure(0, "Failed to create the request"),
    };

    let _ = request.headers().set("Accept", "application/json");

    if request
        .headers()
        .set("Content-Type", "application/json")
        .is_err()
    {}

    if let Some(extra) = headers.and_then(|h| h.as_object()) {
        for (name, value) in extra.iter() {
            let _ = request.headers().set(name, &value.to_string());
        }
    }

    let window = match web_sys::window() {
        Some(w) => w,
        None => return ApiResult::failure(0, "No window available (outside a browser?)"),
    };

    let resp_value = match JsFuture::from(window.fetch_with_request(&request)).await {
        Ok(v) => v,
        Err(_) => {
            return ApiResult::failure(0, "Could not connect to the server");
        }
    };

    let response: Response = match resp_value.dyn_into() {
        Ok(r) => r,
        Err(_) => return ApiResult::failure(0, "Invalid response"),
    };

    let status = response.status();
    let ok = response.ok();

    let text_promise = match response.text() {
        Ok(p) => p,
        Err(_) => return ApiResult::failure(status, "Could not read the response body"),
    };
    let text_value = match JsFuture::from(text_promise).await {
        Ok(v) => v,
        Err(_) => return ApiResult::failure(status, "Could not read the response body"),
    };
    let text = text_value.as_string().unwrap_or_default();

    let data = if text.is_empty() {
        VbValue::Null
    } else {
        VbValue::from_json_str(&text)
    };

    if !ok {
        let nested = data.dig_path("error.message");
        let message = if nested.is_null() {
            data.dig_path("message")
        } else {
            nested
        };
        let error_msg = message
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("HTTP {}", status));
        return ApiResult {
            ok: false,
            status,
            data,
            error: Some(error_msg),
        };
    }

    ApiResult {
        ok: true,
        status,
        data,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_url_relative_path() {
        assert_eq!(
            resolve_url("https://api.vibao.dev", "/users"),
            "https://api.vibao.dev/users"
        );
    }

    #[test]
    fn test_resolve_url_no_leading_slash() {
        assert_eq!(
            resolve_url("https://api.vibao.dev", "users"),
            "https://api.vibao.dev/users"
        );
    }

    #[test]
    fn test_resolve_url_absolute_endpoint_kept_as_is() {
        assert_eq!(
            resolve_url("https://api.vibao.dev", "https://other.com/x"),
            "https://other.com/x"
        );
    }

    #[test]
    fn test_resolve_url_empty_base() {
        assert_eq!(resolve_url("", "/users"), "/users");
    }
}
