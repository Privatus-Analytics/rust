//! # Privatus Analytics for Rust
//!
//! Prints the cookieless [Privatus Analytics](https://privatusanalytics.com)
//! script tag, the attributes for custom events and click maps, and page
//! props, for Rust web apps.
//!
//! The crate only prints HTML for the browser. It never sends events or any
//! other request from your server, sets no cookies, and never reads IP
//! addresses, browser details (user agents) or any visitor id. The core has
//! no dependencies.
//!
//! ```
//! use privatus_analytics::{click_attr, event_attrs, props, Config};
//!
//! let config = Config::new().site_id("pa_YOURSITEID").enabled(true);
//!
//! assert_eq!(
//!     config.tag().to_html().as_str(),
//!     r#"<script defer src="https://privatusstats.com/js/pa.js" data-site="pa_YOURSITEID"></script>"#
//! );
//! assert_eq!(
//!     event_attrs("Report exported", props! { "format" => "csv" }).as_str(),
//!     r#"data-privatus-event="Report exported" data-privatus-prop-format="csv""#
//! );
//! assert_eq!(click_attr("pricing.cta").as_str(), r#"data-privatus-click="pricing.cta""#);
//! ```
//!
//! When [`Config::enabled`] is not set, the tag prints only in release builds
//! (`cfg!(not(debug_assertions))`).
//!
//! ## Cargo features
//!
//! * `axum`: [`axum::PrivatusLayer`], a tower layer that adds the tag to
//!   HTML pages.
//! * `actix`: [`actix::PrivatusMiddleware`], the same for actix-web.
//! * `askama`: askama does not escape [`SafeHtml`] and [`Tag`].
//! * `maud`: [`SafeHtml`] and [`Tag`] implement `maud::Render`, plus
//!   [`maud::element`].
//!
//! See the [README](https://github.com/Privatus-Analytics/rust) and the
//! [Privatus Analytics docs](https://privatusanalytics.com/docs).

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

mod attrs;
mod config;
mod html;
mod page_props;
mod tag;
mod value;

pub use attrs::{
    attrs_to_html, click_attr, click_data, event_attrs, event_data, normalize_key, valid_click,
    MAX_CLICK_NAME, MAX_KEY, MAX_NAME, MAX_PROPS, MAX_VALUE,
};
pub use config::{Config, CspNonce, Spa, UnknownSpaMode, DEFAULT_SCRIPT_HOST};
pub use html::{escape_html, SafeHtml};
pub use page_props::{page_props_json, MAX_ITEMS};
pub use tag::{inject_tag, Tag, MARKER, PROPS_FUNCTION, QUEUE_STUB};
pub use value::{Props, Value};

/// The script tag for these settings. The same as [`Config::tag`].
pub fn tag(config: &Config) -> Tag<'_> {
    config.tag()
}

#[cfg(feature = "actix")]
#[cfg_attr(docsrs, doc(cfg(feature = "actix")))]
pub mod actix;
#[cfg(feature = "askama")]
#[cfg_attr(docsrs, doc(cfg(feature = "askama")))]
pub mod askama;
#[cfg(feature = "axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
pub mod axum;
#[cfg(feature = "maud")]
#[cfg_attr(docsrs, doc(cfg(feature = "maud")))]
pub mod maud;
