use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestTiming {
    pub dns_lookup_ms: u64,
    pub tcp_connect_ms: u64,
    pub tls_handshake_ms: u64,
    pub time_to_first_byte_ms: u64,
    pub download_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ResponseBody {
    #[default]
    Empty,
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseState {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: ResponseBody,
    pub cookies: Vec<Cookie>,
    pub timing: RequestTiming,
    pub size_bytes: usize,
    pub received_at: DateTime<Utc>,
    pub scroll_offset: u16,
}

impl Default for ResponseState {
    fn default() -> Self {
        Self {
            status: 0,
            status_text: String::new(),
            headers: Vec::new(),
            body: ResponseBody::Empty,
            cookies: Vec::new(),
            timing: RequestTiming::default(),
            size_bytes: 0,
            received_at: Utc::now(),
            scroll_offset: 0,
        }
    }
}
