use axum::{
    body::Body,
    extract::Request as AxumRequest,
    response::Response as AxumResponse,
};
use std::collections::HashMap;
use worker::{Request as WorkerRequest, Response as WorkerResponse, Headers};

/// Convert Worker Request to Axum Request without using tokio networking
pub async fn to_axum_request(worker_req: WorkerRequest) -> Result<AxumRequest, worker::Error> {
    let method = worker_req.method();
    let uri = worker_req.url()?.to_string();
    
    // Get headers
    let worker_headers = worker_req.headers();
    let mut axum_headers = axum::http::HeaderMap::new();
    
    // Convert headers
    for (name, value) in worker_headers.entries() {
        if let (Ok(header_name), Ok(header_value)) = (
            axum::http::HeaderName::from_bytes(name.as_bytes()),
            axum::http::HeaderValue::from_str(&value)
        ) {
            axum_headers.insert(header_name, header_value);
        }
    }
    
    // Get body
    let body_bytes = match worker_req.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => Vec::new(),
    };
    
    // Build Axum request
    let mut req_builder = axum::http::Request::builder()
        .method(method)
        .uri(uri);
    
    // Add headers to builder
    let headers_mut = req_builder.headers_mut().unwrap();
    *headers_mut = axum_headers;
    
    // Create body and build request
    let body = Body::from(body_bytes);
    let axum_req = req_builder.body(body).map_err(|e| {
        worker::Error::RustError(format!("Failed to build Axum request: {}", e))
    })?;
    
    Ok(axum_req)
}

/// Convert Axum Response to Worker Response without using tokio networking
pub async fn to_worker_response(axum_resp: AxumResponse) -> Result<WorkerResponse, worker::Error> {
    let (parts, body) = axum_resp.into_parts();
    
    // Convert status
    let status = parts.status.as_u16();
    
    // Convert headers
    let mut worker_headers = Headers::new();
    for (name, value) in parts.headers.iter() {
        if let Ok(value_str) = value.to_str() {
            worker_headers.set(name.as_str(), value_str)?;
        }
    }
    
    // Convert body - collect bytes without using tokio networking
    let body_bytes = match body_to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(e) => {
            worker::console_log!("Error converting body: {}", e);
            Vec::new()
        }
    };
    
    // Create Worker response
    let mut response = WorkerResponse::from_bytes(body_bytes)?;
    
    // Set status and headers
    response = response.with_status(status);
    for (name, value) in worker_headers.entries() {
        response = response.with_headers(Headers::from([(name.as_str(), value.as_str())]));
    }
    
    Ok(response)
}

/// Convert Axum Body to bytes without tokio networking
async fn body_to_bytes(body: Body) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use futures::TryStreamExt;
    
    // Use the body's stream interface instead of tokio
    let mut body_stream = axum::body::BodyExt::into_data_stream(body);
    let mut bytes = Vec::new();
    
    while let Some(chunk) = body_stream.try_next().await? {
        bytes.extend_from_slice(&chunk);
    }
    
    Ok(bytes)
}

/// Oneshot service for handling single requests without tokio's networking
pub async fn oneshot_service<S>(
    service: S, 
    request: AxumRequest
) -> Result<AxumResponse, Box<dyn std::error::Error + Send + Sync>>
where
    S: tower::Service<AxumRequest, Response = AxumResponse> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
{
    use tower::ServiceExt;
    
    // Use tower's ready service without tokio networking
    let mut ready_service = service;
    let response = ready_service.call(request).await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    
    Ok(response)
}