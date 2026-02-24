use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }

    pub fn next(&self) -> HttpMethod {
        match self {
            HttpMethod::Get => HttpMethod::Post,
            HttpMethod::Post => HttpMethod::Put,
            HttpMethod::Put => HttpMethod::Patch,
            HttpMethod::Patch => HttpMethod::Delete,
            HttpMethod::Delete => HttpMethod::Head,
            HttpMethod::Head => HttpMethod::Options,
            HttpMethod::Options => HttpMethod::Get,
        }
    }

    pub fn prev(&self) -> HttpMethod {
        match self {
            HttpMethod::Get => HttpMethod::Options,
            HttpMethod::Post => HttpMethod::Get,
            HttpMethod::Put => HttpMethod::Post,
            HttpMethod::Patch => HttpMethod::Put,
            HttpMethod::Delete => HttpMethod::Patch,
            HttpMethod::Head => HttpMethod::Delete,
            HttpMethod::Options => HttpMethod::Head,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    pub description: String,
}

impl Default for KeyValuePair {
    fn default() -> Self {
        Self {
            key: String::new(),
            value: String::new(),
            enabled: true,
            description: String::new(),
        }
    }
}

impl KeyValuePair {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
            description: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RequestBody {
    #[default]
    None,
    Text(String),
    Json(String),
    Form(Vec<KeyValuePair>),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AuthConfig {
    #[default]
    None,
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { key: String, value: String, in_header: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Scripts {
    pub pre_request: String,
    pub post_response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestState {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub url_cursor: usize,
    pub headers: Vec<KeyValuePair>,
    pub params: Vec<KeyValuePair>,
    pub body: RequestBody,
    pub auth: AuthConfig,
    pub scripts: Scripts,
    #[serde(default)]
    pub body_cursor: usize,
    #[serde(default)]
    pub body_scroll_offset: u16,
}

impl Default for RequestState {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: String::from("Untitled Request"),
            method: HttpMethod::default(),
            url: String::new(),
            url_cursor: 0,
            headers: Vec::new(),
            params: Vec::new(),
            body: RequestBody::None,
            auth: AuthConfig::None,
            scripts: Scripts::default(),
            body_cursor: 0,
            body_scroll_offset: 0,
        }
    }
}
