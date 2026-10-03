use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    Router,
};
use std::sync::OnceLock;
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::state::AppState;

const DIST_DIR: &str = "web/dist";

static INDEX_HTML: OnceLock<String> = OnceLock::new();

pub fn is_available() -> bool {
    std::path::Path::new(DIST_DIR).join("index.html").exists()
}

fn cached_index() -> Option<&'static str> {
    let html = INDEX_HTML.get_or_init(|| {
        std::fs::read_to_string(format!("{DIST_DIR}/index.html")).unwrap_or_default()
    });
    if html.is_empty() { None } else { Some(html.as_str()) }
}

pub fn spa_router() -> Router<AppState> {
    Router::new().fallback(spa_handler)
}

async fn spa_handler(req: Request<Body>) -> Response {
    let path = req.uri().path();

    if path.starts_with("/api/") {
        return StatusCode::NOT_FOUND.into_response();
    }

    if path.contains('.') {
        let is_hashed_asset = path.starts_with("/assets/");

        let static_req = Request::builder()
            .uri(req.uri().clone())
            .body(Body::empty())
            .unwrap();

        let resp = ServeDir::new(DIST_DIR)
            .oneshot(static_req)
            .await
            .into_response();
        if resp.status() != StatusCode::NOT_FOUND {
            if is_hashed_asset {
                let (mut parts, body) = resp.into_parts();
                parts.headers.insert(
                    header::CACHE_CONTROL,
                    "public, max-age=31536000, immutable".parse().unwrap(),
                );
                return Response::from_parts(parts, body);
            }
            return resp;
        }
    }

    match cached_index() {
        Some(html) => Html(html).into_response(),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
