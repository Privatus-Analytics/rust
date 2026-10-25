//! Page props as JSON that is safe inside a `<script>`.

use crate::attrs::{normalize, string_value, MAX_VALUE};
use crate::html::truncate_chars;
use crate::value::Value;
use std::fmt::Write;

/// Lists in page props keep at most this many items.
pub const MAX_ITEMS: usize = 30;

/// Turns page props into a JSON object, or `None` when there is nothing to send.
///
/// Keys follow the same rules as event props. Numbers and booleans stay JSON
/// numbers and booleans, and lists stay JSON arrays (up to 30 items). `<`, `>`,
/// `&`, U+2028 and U+2029 are written as `\u` escapes, so the JSON is safe
/// inside a `<script>` tag.
///
/// ```
/// use privatus_analytics::{page_props_json, props};
///
/// assert_eq!(
///     page_props_json(props! { "plan" => "pro", "seats" => 5 }).as_deref(),
///     Some(r#"{"plan":"pro","seats":5}"#)
/// );
/// assert_eq!(page_props_json(props! {}), None);
/// ```
pub fn page_props_json<I, K, V>(props: I) -> Option<String>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: Into<Value>,
{
    let mut out = String::from("{");
    let mut count = 0;
    for (key, value) in normalize(props) {
        let Some(json) = json_value(&value) else {
            continue;
        };
        if count > 0 {
            out.push(',');
        }
        write_string(&mut out, &key);
        out.push(':');
        out.push_str(&json);
        count += 1;
    }
    if count == 0 {
        return None;
    }
    out.push('}');
    Some(out)
}

fn json_value(value: &Value) -> Option<String> {
    match value {
        Value::List(items) => {
            let parts: Vec<String> = items
                .iter()
                .filter(|item| **item != Value::Null)
                .take(MAX_ITEMS)
                .filter_map(|item| match item {
                    Value::List(_) => Some(json_text(&string_value(item))),
                    other => scalar(other),
                })
                .collect();
            Some(format!("[{}]", parts.join(",")))
        }
        other => scalar(other),
    }
}

fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(b) => Some(b.to_string()),
        Value::Int(n) => Some(n.to_string()),
        Value::UInt(n) => Some(n.to_string()),
        Value::Float(n) if n.is_finite() => Some(format!("{n:?}")),
        Value::Float(_) => None,
        Value::Str(s) => Some(json_text(s)),
        Value::List(_) => Some(json_text(&string_value(value))),
    }
}

fn json_text(text: &str) -> String {
    let mut out = String::new();
    write_string(&mut out, truncate_chars(text, MAX_VALUE));
    out
}

fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '<' | '>' | '&' | '\u{2028}' | '\u{2029}' => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
