//! The conformance example app for the Privatus Analytics Rust crate (axum +
//! askama). The conformance suite
//! (https://github.com/Privatus-Analytics/conformance) builds it and checks
//! every route in a real browser; see its CONTRACT.md. The site id and script
//! host come from PRIVATUS_SITE_ID and PRIVATUS_SCRIPT_HOST.

use askama::Template;
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Router};
use privatus_analytics::axum::PrivatusLayer;
use privatus_analytics::{click_attr, event_attrs, props, Config, CspNonce, Props, SafeHtml};
use std::sync::Arc;

const NONCE: &str = "testnonce";
const NASTY: &str = "</script><script>window.__pwned=1</script>\u{2028}\u{2029}&<>\"'";

const AUTO_PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Auto inject</title>
</head>
<body><h1>Auto inject</h1><p>No tag in the template. The middleware adds it.</p></body>
</html>
"#;

fn page_props() -> Props {
    props! { "plan" => "pro", "role" => "admin", "beta" => true, "seats" => 3 }
}

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
    title: &'static str,
    tag: SafeHtml,
}

#[derive(Template)]
#[template(path = "event.html")]
struct Event {
    title: &'static str,
    tag: SafeHtml,
    export: SafeHtml,
    cta: SafeHtml,
    signup: SafeHtml,
}

#[derive(Template)]
#[template(path = "nasty.html")]
struct Nasty {
    title: &'static str,
    tag: SafeHtml,
    nasty: SafeHtml,
}

#[derive(Template)]
#[template(path = "off.html")]
struct Off {
    tag: SafeHtml,
}

fn render(page: impl Template) -> Response {
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )
            .into_response(),
    }
}

/// The tag for a normal page: page props, the queue stub and the nonce.
fn tag(config: &Config, nonce: &CspNonce, props: Props) -> SafeHtml {
    config
        .tag()
        .props(&props)
        .queue_stub(true)
        .nonce(nonce.0.clone())
        .to_html()
}

async fn home(
    State(config): State<Arc<Config>>,
    Extension(nonce): Extension<CspNonce>,
) -> Response {
    render(Home {
        title: "Home",
        tag: tag(&config, &nonce, page_props()),
    })
}

async fn event(
    State(config): State<Arc<Config>>,
    Extension(nonce): Extension<CspNonce>,
) -> Response {
    render(Event {
        title: "Events",
        tag: tag(&config, &nonce, page_props()),
        export: event_attrs(
            "Report exported",
            props! { "format" => "csv", "source" => "toolbar" },
        ),
        cta: click_attr("pricing.cta"),
        signup: event_attrs("Signup form", props! {}),
    })
}

async fn nasty(
    State(config): State<Arc<Config>>,
    Extension(nonce): Extension<CspNonce>,
) -> Response {
    let mut props = page_props();
    props.insert("nasty", NASTY);
    render(Nasty {
        title: "Nasty",
        tag: tag(&config, &nonce, props),
        nasty: event_attrs("Nasty value", props! { "nasty" => NASTY }),
    })
}

/// Off with the queue stub on prints only the stub, so privatus.track calls
/// still work. Off without it prints nothing.
fn off_page(config: &Config, nonce: &CspNonce, queue_stub: bool) -> Response {
    render(Off {
        tag: config
            .tag()
            .enabled(false)
            .props(&page_props())
            .queue_stub(queue_stub)
            .nonce(nonce.0.clone())
            .to_html(),
    })
}

async fn off(State(config): State<Arc<Config>>, Extension(nonce): Extension<CspNonce>) -> Response {
    off_page(&config, &nonce, true)
}

async fn off_plain(
    State(config): State<Arc<Config>>,
    Extension(nonce): Extension<CspNonce>,
) -> Response {
    off_page(&config, &nonce, false)
}

async fn auto() -> Html<&'static str> {
    Html(AUTO_PAGE)
}

/// Puts the nonce in the request extensions (where the layer reads it) and
/// sends a matching policy.
async fn csp(State(policy): State<HeaderValue>, mut request: Request, next: Next) -> Response {
    request.extensions_mut().insert(CspNonce(NONCE.into()));
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CONTENT_SECURITY_POLICY, policy);
    response
}

#[tokio::main]
async fn main() {
    // Forced on, so the tag prints whatever the build. The site id and script
    // host come from the environment.
    let config = Config::new().enabled(true);
    let policy = HeaderValue::from_str(&format!(
        "script-src 'nonce-{NONCE}' {}",
        config.resolved_script_host()
    ))
    .expect("a valid header value");

    let app = Router::new()
        .route("/", get(home))
        .route("/event", get(event))
        .route("/nasty", get(nasty))
        .route("/off", get(off))
        .route("/off-plain", get(off_plain))
        // Auto inject on this route only, so the other routes test the helpers alone.
        .route("/auto", get(auto).layer(PrivatusLayer::new(config.clone())))
        .with_state(Arc::new(config))
        .layer(middleware::from_fn_with_state(policy, csp));

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("bind the port");
    println!("example app on :{port}");
    axum::serve(listener, app).await.expect("serve");
}
