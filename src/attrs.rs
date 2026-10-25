//! Data attributes the tracker reads for custom events and click maps.

use crate::html::{escape_html, truncate_chars, SafeHtml};
use crate::value::Value;

/// Event names are cut to this many characters.
pub const MAX_NAME: usize = 120;
/// Prop keys are cut to this many characters.
pub const MAX_KEY: usize = 64;
/// At most this many props are kept.
pub const MAX_PROPS: usize = 30;
/// Prop values are cut to this many characters.
pub const MAX_VALUE: usize = 500;
/// Click map names may be at most this many characters.
pub const MAX_CLICK_NAME: usize = 100;

/// The attributes for a custom event, as safe HTML to put inside a tag you write.
///
/// ```
/// use privatus_analytics::{event_attrs, props};
///
/// let attrs = event_attrs("Export CSV", props! { "format" => "csv", "source" => "toolbar" });
/// assert_eq!(
///     attrs.as_str(),
///     r#"data-privatus-event="Export CSV" data-privatus-prop-format="csv" data-privatus-prop-source="toolbar""#
/// );
/// ```
///
/// On a `<form>` the event is sent when the form is submitted. The name is
/// trimmed and cut to 120 characters, and an empty name prints nothing. See
/// [`event_data`] for the rules on props.
///
/// The attributes print even when the tag is off. They do nothing without the
/// tracker, so your pages look the same in development and production.
pub fn event_attrs<I, K, V>(name: &str, props: I) -> SafeHtml
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: Into<Value>,
{
    attrs_to_html(&event_data(name, props))
}

/// The same attributes as [`event_attrs`], as `(name, value)` pairs that are
/// not escaped yet. Handy for template engines and builders that set
/// attributes one by one and escape them for you.
///
/// * Prop keys are made lowercase. Anything that is not `a-z`, `0-9` or `_`
///   becomes `_`, so `"Report Type"` becomes `report_type`. Keys are cut to 64
///   characters.
/// * At most 30 props. Values become text: `true`, `false`, numbers as
///   written, lists joined with commas. `None` and [`Value::Null`] are left
///   out. Values are cut to 500 characters.
///
/// Returns an empty list for an empty name.
pub fn event_data<I, K, V>(name: &str, props: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: Into<Value>,
{
    let name = truncate_chars(name.trim(), MAX_NAME).trim();
    if name.is_empty() {
        return Vec::new();
    }
    let mut attrs = vec![("data-privatus-event".to_owned(), name.to_owned())];
    for (key, value) in normalize(props) {
        let text = string_value(&value);
        attrs.push((
            format!("data-privatus-prop-{key}"),
            truncate_chars(&text, MAX_VALUE).to_owned(),
        ));
    }
    attrs
}

/// The click map name attribute, as safe HTML.
///
/// ```
/// use privatus_analytics::click_attr;
///
/// assert_eq!(click_attr("pricing.cta").as_str(), r#"data-privatus-click="pricing.cta""#);
/// assert!(click_attr("not valid!").is_empty());
/// ```
///
/// A name may use letters, digits and `_ . : -`, up to 100 characters.
/// Anything else prints nothing, and in debug builds a warning goes to
/// standard error.
pub fn click_attr(name: &str) -> SafeHtml {
    attrs_to_html(&click_data(name))
}

/// The click map name as a `(name, value)` pair, or an empty list when the
/// name is not valid.
pub fn click_data(name: &str) -> Vec<(String, String)> {
    if valid_click(name) {
        vec![("data-privatus-click".to_owned(), name.to_owned())]
    } else {
        warn_invalid_click(name);
        Vec::new()
    }
}

/// Whether a click map name is valid: letters, digits and `_ . : -`, from 1 to
/// 100 characters.
pub fn valid_click(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_CLICK_NAME
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'))
}

/// Turns `(name, value)` pairs into HTML attributes, with every value escaped.
pub fn attrs_to_html(attrs: &[(String, String)]) -> SafeHtml {
    let parts: Vec<String> = attrs
        .iter()
        .map(|(name, value)| format!("{name}=\"{}\"", escape_html(value)))
        .collect();
    SafeHtml::from_trusted(parts.join(" "))
}

#[cfg(debug_assertions)]
fn warn_invalid_click(name: &str) {
    eprintln!(
        "[privatus-analytics] click map name {name:?} is not valid. Use letters, digits and \
         _ . : - only, at most 100 characters. Nothing was printed."
    );
}

#[cfg(not(debug_assertions))]
fn warn_invalid_click(_name: &str) {}

/// Cleans a prop key: lowercase, anything outside `a-z 0-9 _` becomes `_`,
/// cut to 64 characters.
pub fn normalize_key(key: &str) -> String {
    key.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(MAX_KEY)
        .collect()
}

/// Cleans keys, drops empty keys and null values, and keeps the first 30 props.
/// A key seen twice keeps its first place and gets the later value.
pub(crate) fn normalize<I, K, V>(props: I) -> Vec<(String, Value)>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: Into<Value>,
{
    let mut clean: Vec<(String, Value)> = Vec::new();
    for (key, value) in props {
        let key = normalize_key(key.as_ref());
        let value = value.into();
        if key.is_empty() || value == Value::Null {
            continue;
        }
        match clean.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => clean.push((key, value)),
        }
    }
    clean.truncate(MAX_PROPS);
    clean
}

/// A value as attribute text. Lists are joined with commas, nulls skipped.
pub(crate) fn string_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::UInt(n) => n.to_string(),
        Value::Float(n) => n.to_string(),
        Value::Str(s) => s.clone(),
        Value::List(items) => items
            .iter()
            .filter(|item| **item != Value::Null)
            .map(string_value)
            .collect::<Vec<_>>()
            .join(","),
    }
}
