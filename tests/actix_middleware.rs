#![cfg(feature = "actix")]

use actix_web::http::header;
use actix_web::{test, web, App, HttpMessage, HttpResponse};
use privatus_analytics::actix::PrivatusMiddleware;
use privatus_analytics::{props, Config, CspNonce, Props};

const PAGE: &str = "<html><head><title>Hi</title></head><body>Hello</body></html>";

fn config() -> Config {
    Config::new().site_id("pa_TEST123456").enabled(true)
}

fn html(body: &'static str) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body)
}

macro_rules! app {
    ($middleware:expr) => {
        test::init_service(
            App::new()
                .wrap($middleware)
                .route("/", web::get().to(|| async { html(PAGE) }))
                .route(
                    "/json",
                    web::get().to(|| async {
                        HttpResponse::Ok()
                            .content_type("application/json")
                            .body("{}")
                    }),
                )
                .route(
                    "/missing",
                    web::get().to(|| async {
                        HttpResponse::NotFound()
                            .content_type("text/html")
                            .body(PAGE)
                    }),
                )
                .route(
                    "/gzip",
                    web::get().to(|| async {
                        HttpResponse::Ok()
                            .content_type("text/html")
                            .insert_header((header::CONTENT_ENCODING, "gzip"))
                            .body(PAGE)
                    }),
                )
                .route(
                    "/tagged",
                    web::get().to(|| async {
                        html("<html><head><script src=\"/js/pa.js\"></script></head></html>")
                    }),
                )
                .route(
                    "/billing",
                    web::get().to(|| async {
                        let mut response = html(PAGE);
                        response
                            .extensions_mut()
                            .insert(props! { "section" => "billing" });
                        response
                    }),
                ),
        )
        .await
    };
}

#[actix_web::test]
async fn adds_the_tag_before_head() {
    let app = app!(PrivatusMiddleware::new(config()));
    let response = test::call_service(&app, test::TestRequest::get().uri("/").to_request()).await;
    let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
    assert!(body.contains(
        "<script defer src=\"https://privatusstats.com/js/pa.js\" data-site=\"pa_TEST123456\"></script>\n</head>"
    ));
}

#[actix_web::test]
async fn leaves_other_responses_alone() {
    let app = app!(PrivatusMiddleware::new(config()));
    for path in ["/json", "/missing", "/gzip", "/tagged"] {
        let response =
            test::call_service(&app, test::TestRequest::get().uri(path).to_request()).await;
        let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
        assert!(!body.contains("pa_TEST123456"), "{path}: {body}");
    }
}

#[actix_web::test]
async fn skips_ajax_requests_and_disabled_configs() {
    let app = app!(PrivatusMiddleware::new(config()));
    let request = test::TestRequest::get()
        .uri("/")
        .insert_header(("X-Requested-With", "XMLHttpRequest"))
        .to_request();
    let body = test::call_and_read_body(&app, request).await;
    assert_eq!(body, PAGE.as_bytes());

    let app =
        app!(PrivatusMiddleware::new(config().enabled(false)).props_fn(|_| panic!("must not run")));
    let body = test::call_and_read_body(&app, test::TestRequest::get().uri("/").to_request()).await;
    assert_eq!(body, PAGE.as_bytes());

    let app = app!(PrivatusMiddleware::new(config().auto_inject(false)));
    let body = test::call_and_read_body(&app, test::TestRequest::get().uri("/").to_request()).await;
    assert_eq!(body, PAGE.as_bytes());
}

#[actix_web::test]
async fn uses_props_fn_extension_props_and_the_nonce() {
    let middleware = PrivatusMiddleware::new(config().props(props! { "app_version" => "2.4" }))
        .props_fn(|request| -> Props {
            props! { "path" => request.path().to_owned() }
        });
    let app = app!(middleware);
    let request = test::TestRequest::get().uri("/billing").to_request();
    request.extensions_mut().insert(CspNonce("n0nce".into()));
    let body = test::call_and_read_body(&app, request).await;
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        body.contains(r#"<script nonce="n0nce">window.privatusPageProps=function(){return {"app_version":"2.4","path":"/billing","section":"billing"}}</script>"#),
        "{body}"
    );
}
