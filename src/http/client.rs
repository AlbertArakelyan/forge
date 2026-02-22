use reqwest::Client;
use std::time::Duration;

pub fn build_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("Failed to build HTTP client")
}
