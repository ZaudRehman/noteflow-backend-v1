use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct RequestId(pub String);

pub async fn request_id_middleware(
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    let request_id = Uuid::new_v4().to_string();
    req.extensions_mut()
        .insert(RequestId(request_id.clone()));

    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        user_id = tracing::field::Empty,
    );

    let mut response = span.in_scope(|| next.run(req)).await;
    response
        .headers_mut()
        .insert("X-Request-Id", HeaderValue::from_str(&request_id).unwrap());

    Ok(response)
}
