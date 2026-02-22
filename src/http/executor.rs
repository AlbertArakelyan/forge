use std::time::Instant;
use chrono::Utc;
use reqwest::Client;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::event::Event;
use crate::state::request_state::RequestState;
use crate::state::response_state::{Cookie, RequestTiming, ResponseBody, ResponseState};
use super::builder::build_request;

pub async fn execute(
    client: Client,
    request: RequestState,
    tx: UnboundedSender<Event>,
    cancel: CancellationToken,
) {
    let result = tokio::select! {
        res = do_execute(client, request) => res,
        _ = cancel.cancelled() => Err(AppError::Cancelled),
    };
    let _ = tx.send(Event::Response(result));
}

async fn do_execute(client: Client, state: RequestState) -> Result<ResponseState, AppError> {
    let start = Instant::now();

    let builder = build_request(&client, &state)?;
    let request = builder.build().map_err(AppError::Http)?;
    let response = client.execute(request).await?;

    let ttfb_ms = start.elapsed().as_millis() as u64;

    let status = response.status();
    let status_code = status.as_u16();
    let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();

    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Parse cookies from Set-Cookie headers
    let cookies: Vec<Cookie> = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|header| parse_set_cookie(header))
        .collect();

    let bytes = response.bytes().await?;
    let download_ms = start.elapsed().as_millis() as u64 - ttfb_ms;
    let total_ms = start.elapsed().as_millis() as u64;
    let size_bytes = bytes.len();

    let body = if content_type.contains("application/json") {
        match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(json) => ResponseBody::Text(serde_json::to_string_pretty(&json)?),
            Err(_) => ResponseBody::Text(String::from_utf8_lossy(&bytes).into_owned()),
        }
    } else if content_type.contains("text/")
        || content_type.contains("application/xml")
        || content_type.contains("application/xhtml")
        || content_type.contains("application/javascript")
    {
        ResponseBody::Text(String::from_utf8_lossy(&bytes).into_owned())
    } else if bytes.is_empty() {
        ResponseBody::Empty
    } else {
        match std::str::from_utf8(&bytes) {
            Ok(text) => ResponseBody::Text(text.to_string()),
            Err(_) => ResponseBody::Binary(bytes.to_vec()),
        }
    };

    Ok(ResponseState {
        status: status_code,
        status_text,
        headers,
        body,
        cookies,
        timing: RequestTiming {
            dns_lookup_ms: 0,
            tcp_connect_ms: 0,
            tls_handshake_ms: 0,
            time_to_first_byte_ms: ttfb_ms,
            download_ms,
            total_ms,
        },
        size_bytes,
        received_at: Utc::now(),
        scroll_offset: 0,
        highlighted_body: None, // computed by app.rs once the response arrives
    })
}

/// Minimal Set-Cookie header parser.
fn parse_set_cookie(header: &str) -> Cookie {
    let mut parts = header.splitn(2, ';');
    let name_value = parts.next().unwrap_or("");
    let mut nv = name_value.splitn(2, '=');
    let name = nv.next().unwrap_or("").trim().to_string();
    let value = nv.next().unwrap_or("").trim().to_string();

    let mut domain = String::new();
    let mut path = "/".to_string();
    for attr in parts.next().unwrap_or("").split(';') {
        let attr = attr.trim();
        if let Some(d) = attr.strip_prefix("Domain=") {
            domain = d.to_string();
        } else if let Some(p) = attr.strip_prefix("Path=") {
            path = p.to_string();
        }
    }
    Cookie { name, value, domain, path }
}
