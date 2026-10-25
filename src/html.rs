use std::fmt;

/// HTML that is already escaped and safe to print as it is.
///
/// Every helper in this crate returns `SafeHtml`. Printing it with `{}` writes
/// the HTML without escaping it again. With the `askama` feature it is not
/// escaped by askama, and with the `maud` feature it implements
/// [`maud::Render`](https://docs.rs/maud/latest/maud/trait.Render.html).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SafeHtml(String);

impl SafeHtml {
    /// Wraps a string you know is safe HTML. Nothing is escaped.
    pub fn from_trusted(html: impl Into<String>) -> Self {
        SafeHtml(html.into())
    }

    /// The HTML as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The HTML as an owned string.
    pub fn into_string(self) -> String {
        self.0
    }

    /// Whether there is nothing to print.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for SafeHtml {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for SafeHtml {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<SafeHtml> for String {
    fn from(html: SafeHtml) -> String {
        html.0
    }
}

/// Escapes text for an HTML attribute value or element body.
///
/// `&`, `<`, `>`, `"` and `'` become `&amp;`, `&lt;`, `&gt;`, `&quot;` and `&#39;`.
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Cuts a string to at most `max` characters (not bytes).
pub(crate) fn truncate_chars(text: &str, max: usize) -> &str {
    match text.char_indices().nth(max) {
        Some((index, _)) => &text[..index],
        None => text,
    }
}
