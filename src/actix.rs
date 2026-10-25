//! An actix-web middleware that adds the tag to the `<head>` of HTML pages.
//!
//! ```no_run
//! use actix_web::{web, App, HttpResponse};
//! use privatus_analytics::{actix::PrivatusMiddleware, Config};
//!
//! let config = Config::new().site_id("pa_YOURSITEID");
//! let app = App::new()
//!     .wrap(PrivatusMiddleware::new(config))
//!     .route("/", web::get().to(|| async {
//!         HttpResponse::Ok()
//!             .content_type("text/html; charset=utf-8")
//!             .body("<html><head></head><body>Hi</body></html>")
//!     }));
//! # let _ = app;
//! ```
//!
//! The middleware adds the tag just before `</head>` in uncompressed
//! `text/html` responses with status 200. It skips Ajax requests
//! (`X-Requested-With: XMLHttpRequest`), pages without a `</head>`, and pages
//! that already have the tag. Put it inside (after) `Compress` in your
//! `wrap` calls, or it sees compressed pages and leaves them alone.
//!
//! It reads a [`CspNonce`] and page [`Props`] from the request extensions or
//! the response extensions, so your own middleware or a handler can set them.

use crate::config::{Config, CspNonce};
use crate::tag::{inject_tag, should_inject};
use crate::value::Props;
use actix_web::body::{to_bytes, EitherBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header;
use actix_web::web::Bytes;
use actix_web::{Error, HttpMessage, HttpRequest};
use std::fmt;
use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};

type PropsFn = dyn Fn(&HttpRequest) -> Props;

/// The middleware that adds the tag. Add it with `App::wrap`.
#[derive(Clone)]
pub struct PrivatusMiddleware {
    config: Arc<Config>,
    props_fn: Option<Rc<PropsFn>>,
}

impl PrivatusMiddleware {
    /// A middleware with these settings.
    pub fn new(config: Config) -> Self {
        Self::from_arc(Arc::new(config))
    }

    /// A middleware that shares settings you already keep in an `Arc`.
    pub fn from_arc(config: Arc<Config>) -> Self {
        PrivatusMiddleware {
            config,
            props_fn: None,
        }
    }

    /// Page props worked out for each request. The function gets the
    /// request. It runs only when the tag is added to the page.
    ///
    /// Never return names, emails, user ids or anything else that points to
    /// one person.
    pub fn props_fn<F>(mut self, props_fn: F) -> Self
    where
        F: Fn(&HttpRequest) -> Props + 'static,
    {
        self.props_fn = Some(Rc::new(props_fn));
        self
    }
}

impl fmt::Debug for PrivatusMiddleware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivatusMiddleware")
            .field("config", &self.config)
            .field("props_fn", &self.props_fn.is_some())
            .finish()
    }
}

impl<S, B> Transform<S, ServiceRequest> for PrivatusMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = PrivatusMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PrivatusMiddlewareService {
            service,
            config: self.config.clone(),
            props_fn: self.props_fn.clone(),
        }))
    }
}

/// The service made by [`PrivatusMiddleware`].
pub struct PrivatusMiddlewareService<S> {
    service: S,
    config: Arc<Config>,
    props_fn: Option<Rc<PropsFn>>,
}

impl<S> fmt::Debug for PrivatusMiddlewareService<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrivatusMiddlewareService")
            .field("config", &self.config)
            .finish()
    }
}

impl<S, B> Service<ServiceRequest> for PrivatusMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let config = self.config.clone();
        let props_fn = self.props_fn.clone();
        let xhr = request
            .headers()
            .get("x-requested-with")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let future = self.service.call(request);

        Box::pin(async move {
            let response = future.await?;
            let headers = response.headers();
            let content_type = headers
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok());
            let encoded = headers.contains_key(header::CONTENT_ENCODING);
            let inject = config.is_auto_inject()
                && should_inject(
                    response.status().as_u16(),
                    content_type,
                    encoded,
                    xhr.as_deref(),
                )
                && config.tag().will_render();
            if !inject {
                return Ok(response.map_into_left_body());
            }

            let (request, response) = response.into_parts();
            let mut tag = config.tag();
            if let Some(props_fn) = &props_fn {
                tag = tag.props(props_fn(&request));
            }
            if let Some(props) = request.extensions().get::<Props>() {
                tag = tag.props(props);
            }
            if let Some(props) = response.extensions().get::<Props>() {
                tag = tag.props(props);
            }
            let nonce = response
                .extensions()
                .get::<CspNonce>()
                .cloned()
                .or_else(|| request.extensions().get::<CspNonce>().cloned());
            if let Some(nonce) = nonce {
                tag = tag.nonce(nonce.0);
            }
            let tag = tag.to_html();

            let (mut response, body) = response.into_parts();
            let bytes = to_bytes(body)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e.into().to_string()))?;
            let bytes = std::str::from_utf8(&bytes)
                .ok()
                .and_then(|page| inject_tag(page, tag.as_str()))
                .map_or(bytes, Bytes::from);
            response.headers_mut().remove(header::CONTENT_LENGTH);
            let response = response.set_body(bytes).map_into_boxed_body();
            Ok(ServiceResponse::new(request, response).map_into_right_body())
        })
    }
}
