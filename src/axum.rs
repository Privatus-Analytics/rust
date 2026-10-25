//! A tower layer for axum that adds the tag to the `<head>` of HTML pages.
//!
//! ```no_run
//! use axum::{response::Html, routing::get, Router};
//! use privatus_analytics::{axum::PrivatusLayer, props, Config};
//!
//! let config = Config::new().site_id("pa_YOURSITEID");
//! let app: Router = Router::new()
//!     .route("/", get(|| async { Html("<html><head></head><body>Hi</body></html>") }))
//!     .layer(PrivatusLayer::new(config).props_fn(|_request| props! { "app_version" => "2.4" }));
//! ```
//!
//! The layer adds the tag just before `</head>` in uncompressed `text/html`
//! responses with status 200. It skips Ajax requests
//! (`X-Requested-With: XMLHttpRequest`), pages without a `</head>`, and pages
//! that already have the tag. It fixes the `Content-Length` header.
//!
//! It reads a [`CspNonce`] and page [`Props`] from the request extensions or
//! the response extensions, so your own middleware or a handler can set them.

use crate::config::{Config, CspNonce};
use crate::tag::{inject_tag, should_inject};
use crate::value::Props;
use axum::body::{Body, Bytes, HttpBody};
use axum::http::{header, request::Parts, HeaderValue, Request, Response, StatusCode};
use axum::BoxError;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

type PropsFn = dyn Fn(&Parts) -> Props + Send + Sync;

/// The tower [`Layer`](tower_layer::Layer) that adds the tag. Add it with
/// `Router::layer`.
#[derive(Clone)]
pub struct PrivatusLayer {
    config: Arc<Config>,
    props_fn: Option<Arc<PropsFn>>,
}

impl PrivatusLayer {
    /// A layer with these settings.
    pub fn new(config: Config) -> Self {
        Self::from_arc(Arc::new(config))
    }

    /// A layer that shares settings you already keep in an `Arc`.
    pub fn from_arc(config: Arc<Config>) -> Self {
        PrivatusLayer {
            config,
            props_fn: None,
        }
    }

    /// Page props worked out for each request. The function gets the request
    /// parts (path, headers, extensions). It runs only when the tag is on.
    ///
    /// Never return names, emails, user ids or anything else that points to
    /// one person.
    pub fn props_fn<F>(mut self, props_fn: F) -> Self
    where
        F: Fn(&Parts) -> Props + Send + Sync + 'static,
    {
        self.props_fn = Some(Arc::new(props_fn));
        self
    }
}

impl fmt::Debug for PrivatusLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivatusLayer")
            .field("config", &self.config)
            .field("props_fn", &self.props_fn.is_some())
            .finish()
    }
}

impl<S> tower_layer::Layer<S> for PrivatusLayer {
    type Service = PrivatusService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        PrivatusService {
            inner,
            layer: self.clone(),
        }
    }
}

/// The service made by [`PrivatusLayer`].
#[derive(Clone, Debug)]
pub struct PrivatusService<S> {
    inner: S,
    layer: PrivatusLayer,
}

struct RequestInfo {
    props: Option<Props>,
    extension_props: Option<Props>,
    nonce: Option<CspNonce>,
}

impl<S, ReqBody, ResBody> tower_service::Service<Request<ReqBody>> for PrivatusService<S>
where
    S: tower_service::Service<Request<ReqBody>, Response = Response<ResBody>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: HttpBody<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError>,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        let config = self.layer.config.clone();
        let xhr = request
            .headers()
            .get("x-requested-with")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("XMLHttpRequest"));
        let active = !xhr && config.is_auto_inject() && config.tag().will_render();

        let (parts, body) = request.into_parts();
        let info = active.then(|| RequestInfo {
            props: self.layer.props_fn.as_ref().map(|f| f(&parts)),
            extension_props: parts.extensions.get::<Props>().cloned(),
            nonce: parts.extensions.get::<CspNonce>().cloned(),
        });
        let request = Request::from_parts(parts, body);

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            let response = inner.call(request).await?;
            let Some(info) = info else {
                return Ok(response.map(Body::new));
            };
            let headers = response.headers();
            let content_type = headers
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok());
            let encoded = headers.contains_key(header::CONTENT_ENCODING);
            if !should_inject(response.status().as_u16(), content_type, encoded, None) {
                return Ok(response.map(Body::new));
            }

            let (mut parts, body) = response.into_parts();
            let bytes = match axum::body::to_bytes(Body::new(body), usize::MAX).await {
                Ok(bytes) => bytes,
                Err(_) => {
                    // The handler's body failed while we read it. Nothing sensible is left to send.
                    let mut failed = Response::new(Body::empty());
                    *failed.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    return Ok(failed);
                }
            };

            let html = std::str::from_utf8(&bytes).ok().and_then(|page| {
                let mut tag = config.tag();
                if let Some(props) = &info.props {
                    tag = tag.props(props);
                }
                if let Some(props) = &info.extension_props {
                    tag = tag.props(props);
                }
                if let Some(props) = parts.extensions.get::<Props>() {
                    tag = tag.props(props);
                }
                if let Some(nonce) = parts.extensions.get::<CspNonce>().or(info.nonce.as_ref()) {
                    tag = tag.nonce(nonce.0.clone());
                }
                inject_tag(page, tag.to_html().as_str())
            });
            let bytes = html.map_or(bytes, Bytes::from);
            parts
                .headers
                .insert(header::CONTENT_LENGTH, HeaderValue::from(bytes.len()));
            Ok(Response::from_parts(parts, Body::from(bytes)))
        })
    }
}
