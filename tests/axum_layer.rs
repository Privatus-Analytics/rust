#![cfg(feature = "axum")]

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Router};
use privatus_analytics::axum::PrivatusLayer;
use privatus_analytics::{props, Config, CspNonce};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tower::ServiceExt;

const PAGE: &str = "<html><head><title>Hi</title></head><body>Hello</body></html>";

fn config() -> Config {
    Config::new().site_id("pa_TEST123456").enabled(true)
}

fn app(layer: PrivatusLayer) -> Router {
    Router::new()
        .route("/", get(|| async { Html(PAGE) }))
        .route(
            "/json",
            get(|| async { ([(header::CONTENT_TYPE, "application/json")], "{}") }),
        )
        .route(
            "/missing",
            get(|| async { (StatusCode::NOT_FOUND, Html(PAGE)) }),
        )
        .route(
            "/gzip",
            get(|| async { ([(header::CONTENT_ENCODING, "gzip")], Html(PAGE)) }),
        )
        .route(
            "/tagged",
            get(|| async { Html("<html><head><script src=\"/js/pa.js\"></script></head></html>") }),
        )
        .route(
            "/billing",
            get(|| async {
                (Extension(props! { "section" => "billing" }), Html(PAGE)).into_response()
            }),
        )
        .layer(layer)
}

async fn fetch(app: Router, request: Request<Body>) -> (Response, String) {
    let response = app.oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    (Response::from_parts(parts, Body::empty()), text)
}

fn get_request(path: &str) -> Request<Body> {
    Request::get(path).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn adds_the_tag_before_head_and_fixes_content_length() {
    let (response, body) = fetch(app(PrivatusLayer::new(config())), get_request("/")).await;
    assert!(body.contains(
        "<script defer src=\"https://privatusstats.com/js/pa.js\" data-site=\"pa_TEST123456\"></script>\n</head>"
    ));
    assert_eq!(
        response.headers()[header::CONTENT_LENGTH].to_str().unwrap(),
        body.len().to_string()
    );
}

#[tokio::test]
async fn leaves_other_responses_alone() {
    for path in ["/json", "/missing", "/gzip", "/tagged"] {
        let (_, body) = fetch(app(PrivatusLayer::new(config())), get_request(path)).await;
        assert!(!body.contains("pa_TEST123456"), "{path}: {body}");
    }
}

#[tokio::test]
async fn skips_ajax_requests() {
    let request = Request::get("/")
        .header("X-Requested-With", "XMLHttpRequest")
        .body(Body::empty())
        .unwrap();
    let (_, body) = fetch(app(PrivatusLayer::new(config())), request).await;
    assert_eq!(body, PAGE);
}

#[tokio::test]
async fn does_nothing_when_disabled_or_auto_inject_is_off() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let layer = PrivatusLayer::new(config().enabled(false)).props_fn(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
        props! {}
    });
    let (_, body) = fetch(app(layer), get_request("/")).await;
    assert_eq!(body, PAGE);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "props_fn must not run when the tag is off"
    );

    let (_, body) = fetch(
        app(PrivatusLayer::new(config().auto_inject(false))),
        get_request("/"),
    )
    .await;
    assert_eq!(body, PAGE);

    // Off with the queue stub on still adds nothing, not even the stub.
    let (_, body) = fetch(
        app(PrivatusLayer::new(config().enabled(false).queue_stub(true))),
        get_request("/"),
    )
    .await;
    assert_eq!(body, PAGE);
}

#[tokio::test]
async fn uses_props_fn_extension_props_and_the_nonce() {
    let layer = PrivatusLayer::new(config().props(props! { "app_version" => "2.4" }))
        .props_fn(|parts| props! { "path_group" => if parts.uri.path().starts_with("/billing") { "billing" } else { "other" } });
    let request = Request::get("/billing")
        .extension(CspNonce("n0nce".into()))
        .body(Body::empty())
        .unwrap();
    let (_, body) = fetch(app(layer), request).await;
    assert!(
        body.contains(r#"<script nonce="n0nce">window.privatusPageProps=function(){return {"app_version":"2.4","path_group":"billing","section":"billing"}}</script>"#),
        "{body}"
    );
    assert!(
        body.contains(r#"nonce="n0nce" data-props="privatusPageProps""#),
        "{body}"
    );
}
