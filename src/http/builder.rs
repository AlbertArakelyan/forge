use reqwest::{Client, Method, RequestBuilder};
use crate::error::AppError;
use crate::state::request_state::{AuthConfig, HttpMethod, RequestBody, RequestState};

/// Normalize a bare URL into a fully-qualified one.
/// - `:3000/path` → `http://localhost:3000/path`
/// - `localhost/...` → `http://localhost/...`
/// - anything else without a scheme → `https://...`
pub fn normalize_url(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        return url.to_string();
    }
    if url.starts_with(':') {
        return format!("http://localhost{}", url);
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    if url.starts_with("localhost") || url.starts_with("127.0.0.1") {
        return format!("http://{}", url);
    }
    format!("https://{}", url)
}

pub fn build_request(client: &Client, state: &RequestState) -> Result<RequestBuilder, AppError> {
    let method = match &state.method {
        HttpMethod::Get => Method::GET,
        HttpMethod::Post => Method::POST,
        HttpMethod::Put => Method::PUT,
        HttpMethod::Patch => Method::PATCH,
        HttpMethod::Delete => Method::DELETE,
        HttpMethod::Head => Method::HEAD,
        HttpMethod::Options => Method::OPTIONS,
    };

    let url = normalize_url(&state.url);
    let mut builder = client.request(method, &url);

    for param in &state.params {
        if param.enabled && !param.key.is_empty() {
            builder = builder.query(&[(&param.key, &param.value)]);
        }
    }

    for header in &state.headers {
        if header.enabled && !header.key.is_empty() {
            builder = builder.header(&header.key, &header.value);
        }
    }

    builder = match &state.auth {
        AuthConfig::None => builder,
        AuthConfig::Bearer { token } => builder.bearer_auth(token),
        AuthConfig::Basic { username, password } => builder.basic_auth(username, Some(password)),
        AuthConfig::ApiKey { key, value, in_header } => {
            if *in_header {
                builder.header(key.as_str(), value.as_str())
            } else {
                builder.query(&[(key.as_str(), value.as_str())])
            }
        }
    };

    builder = match &state.body {
        RequestBody::None => builder,
        RequestBody::Text(text) => builder
            .body(text.clone())
            .header("Content-Type", "text/plain"),
        RequestBody::Json(json) => builder
            .body(json.clone())
            .header("Content-Type", "application/json"),
        RequestBody::Form(pairs) => {
            let form_pairs: Vec<(String, String)> = pairs
                .iter()
                .filter(|p| p.enabled)
                .map(|p| (p.key.clone(), p.value.clone()))
                .collect();
            builder.form(&form_pairs)
        }
        RequestBody::Binary(bytes) => builder.body(bytes.clone()),
    };

    Ok(builder)
}
