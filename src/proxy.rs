use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request, Response, StatusCode},
    response::IntoResponse,
    routing::any,
    Router,
};
use crate::compressor::SchemaCompressor;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProxyState {
    pub upstream_url: String,
    pub compressor: Arc<SchemaCompressor>,
}

pub async fn start_proxy_server(port: u16, upstream: String) {
    let state = ProxyState {
        upstream_url: upstream.clone(),
        compressor: Arc::new(SchemaCompressor::with_default()),
    };

    let app = Router::new()
        .fallback(any(proxy_handler))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    println!("\n🚀 schemashrink Zero-Config Transparent Proxy active on http://{}", addr);
    println!("📡 Intercepting & auto-shrinking tool schemas -> forwarding to: {}\n", upstream);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn proxy_handler(
    State(state): State<ProxyState>,
    method: Method,
    headers: HeaderMap,
    req: Request<Body>,
) -> impl IntoResponse {
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_default();

    let is_chat_completion = method == Method::POST && path_and_query.contains("/chat/completions");
    let target_url = format!("{}{}", state.upstream_url.trim_end_matches('/'), path_and_query);

    // Read body bytes
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!("Failed to read request body: {}", e)))
                .unwrap();
        }
    };

    // If POST to chat/completions, intercept & shrink the `tools` array on the fly!
    let mut final_body = body_bytes.to_vec();
    if is_chat_completion {
        if let Ok(mut json_val) = serde_json::from_slice::<Value>(&body_bytes) {
            if let Some(tools) = json_val.get_mut("tools") {
                let orig_len = serde_json::to_string(tools).unwrap_or_default().len();
                let shrunk_tools = state.compressor.compress_value(tools);
                let shrunk_len = serde_json::to_string(&shrunk_tools).unwrap_or_default().len();

                *tools = shrunk_tools;

                if orig_len > shrunk_len {
                    let saved_pct = ((orig_len - shrunk_len) as f64 / orig_len as f64) * 100.0;
                    println!(
                        "⚡ [schemashrink] Auto-compressed tool schemas: {} -> {} chars (-{:.1}%)",
                        orig_len, shrunk_len, saved_pct
                    );
                }

                if let Ok(new_bytes) = serde_json::to_vec(&json_val) {
                    final_body = new_bytes;
                }
            }
        }
    }

    // Forward request to upstream API
    let client = reqwest::Client::new();
    let mut req_builder = client.request(method, &target_url);

    for (k, v) in headers.iter() {
        if k != "host" && k != "content-length" {
            req_builder = req_builder.header(k, v);
        }
    }

    req_builder = req_builder.body(final_body);

    match req_builder.send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::OK);
            let mut response_builder = Response::builder().status(status);

            for (k, v) in resp.headers().iter() {
                response_builder = response_builder.header(k, v);
            }

            let stream = resp.bytes_stream();
            response_builder.body(Body::from_stream(stream)).unwrap()
        }
        Err(err) => Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::from(format!("Upstream proxy error: {}", err)))
            .unwrap(),
    }
}
