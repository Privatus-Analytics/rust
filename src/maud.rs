//! maud support: [`SafeHtml`] and [`Tag`] implement [`maud::Render`].
//!
//! ```
//! use maud::html;
//! use privatus_analytics::{click_attr, event_attrs, maud::element, props, Config};
//!
//! let config = Config::new().site_id("pa_YOURSITEID").enabled(true);
//! let page = html! {
//!     head { (config.tag()) }
//!     body {
//!         (element("button", &event_attrs("Report exported", props! { "format" => "csv" }), html! { "Export" }))
//!         (element("a", &click_attr("pricing.cta"), "See plans"))
//!     }
//! };
//! assert!(page.into_string().contains(r#"<button data-privatus-event="Report exported" data-privatus-prop-format="csv">Export</button>"#));
//! ```
//!
//! maud does not let you add attributes from a string inside an element, so
//! use [`element`] to wrap content in a tag with the attributes. When the
//! names are fixed you can also write them in maud as usual:
//! `button data-privatus-event="Report exported" { "Export" }`.

use crate::html::{escape_html, SafeHtml};
use crate::tag::Tag;
use maud::{Markup, PreEscaped, Render};

impl Render for SafeHtml {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(self.as_str());
    }
}

impl Render for Tag<'_> {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(self.to_html().as_str());
    }
}

/// An element with the given attributes around `content`, for example
/// `element("button", &event_attrs(...), "Export")`.
///
/// `name` must be a plain tag name (letters, digits and `-`). Any other name
/// renders the content with no tag around it.
pub fn element(name: &str, attrs: &SafeHtml, content: impl Render) -> Markup {
    let valid = !name.is_empty()
        && name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    let content = content.render().into_string();
    if !valid {
        return PreEscaped(content);
    }
    let name = escape_html(name);
    let space = if attrs.is_empty() { "" } else { " " };
    PreEscaped(format!("<{name}{space}{attrs}>{content}</{name}>"))
}
