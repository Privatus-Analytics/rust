//! askama support: [`SafeHtml`] and [`Tag`] are not escaped by askama.
//!
//! ```
//! use askama::Template;
//! use privatus_analytics::{click_attr, event_attrs, props, Config, SafeHtml};
//!
//! #[derive(Template)]
//! #[template(
//!     source = "<head>{{ tag }}</head><button {{ export }}>Export</button><a {{ cta }}>Plans</a>",
//!     ext = "html"
//! )]
//! struct Page {
//!     tag: SafeHtml,
//!     export: SafeHtml,
//!     cta: SafeHtml,
//! }
//!
//! let config = Config::new().site_id("pa_YOURSITEID").enabled(true);
//! let page = Page {
//!     tag: config.tag().to_html(),
//!     export: event_attrs("Report exported", props! { "format" => "csv" }),
//!     cta: click_attr("pricing.cta"),
//! };
//! let html = page.render().unwrap();
//! assert!(html.contains(r#"<button data-privatus-event="Report exported" data-privatus-prop-format="csv">"#));
//! ```
//!
//! askama says it may still escape a marked type in rare cases. If you ever
//! see `&lt;script` in your page, add the `|safe` filter: `{{ tag|safe }}`.

use crate::html::SafeHtml;
use crate::tag::Tag;

impl askama::filters::HtmlSafe for SafeHtml {}
impl askama::filters::HtmlSafe for Tag<'_> {}
