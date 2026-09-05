// ============================================================
// VIBAO RUNTIME (Rust/WASM) — runtime/api.rs
// A port of the core of 19-runtime-api.ts: makes a real fetch call via
// web-sys, returning a normalized result so action.rs can read __ok and
// branch into thanh_cong/that_bai. Never panics outward on a
// network/HTTP error - always returns an ApiResult, so a
// "that_bai { ... }" action block always runs.
//
// Not yet ported: __auth (automatically attaching an auth token header)
// - left for a later round, since ViBao currently has no auth
// declaration syntax in the language (guard(...) hasn't appeared in
// ast.rs), so there's nothing yet to hook this into.
// ============================================================

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

use crate::runtime::value::VbValue;

/// The result of an API call, normalized whether it succeeded or
/// failed - equivalent to { __ok, status, data, error } in the old JS
/// version.
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

/// Joins an endpoint with the base URL, equivalent to
/// __api.resolveURL. If the endpoint is already a full URL
/// (http/https), it's kept as-is.
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

/// Makes a real API call via `fetch` (web-sys). Equivalent to
/// __api.call(method, endpoint, data). `data` (if present) is
/// serialized to JSON and sent as the request body - skipped for
/// GET/DELETE, matching the old JS version's behavior (these 2 methods
/// typically don't have a body under REST convention).
pub async fn call(base_url: &str, method: &str, endpoint: &str, data: Option<&VbValue>) -> ApiResult {
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

    if request.headers().set("Content-Type", "application/json").is_err() {
        // Doesn't block continuing if setting the header fails - still
        // attempts the request, matching the old JS version's "don't
        // throw outward" spirit.
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

    // Reads the body as text and then parses JSON itself - this is
    // simpler and safer than calling Response's .json() (which throws
    // if the body isn't valid JSON); here the fallback to a plain Str
    // is decided explicitly.
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
        let error_msg = data
            .as_object()
            .and_then(|o| o.get("message"))
            .and_then(|v| v.as_str())
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
        assert_eq!(resolve_url("https://api.vibao.dev", "/users"), "https://api.vibao.dev/users");
    }

    #[test]
    fn test_resolve_url_no_leading_slash() {
        assert_eq!(resolve_url("https://api.vibao.dev", "users"), "https://api.vibao.dev/users");
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
