use crate::attrs::normalize_key;
use crate::config::{Config, Spa};
use crate::html::{escape_html, SafeHtml};
use crate::page_props::page_props_json;
use crate::value::Value;
use std::borrow::Cow;
use std::fmt;

/// The name of the page props function the tracker calls (`data-props`).
pub const PROPS_FUNCTION: &str = "privatusPageProps";

/// The tracker's official queue stub. It keeps `privatus.track` and the other
/// calls in a list until `pa.js` has loaded.
pub const QUEUE_STUB: &str = "window.privatus=window.privatus||{q:[]};['track','pageview','props','optOut','optIn','ready'].forEach(function(m){window.privatus[m]=window.privatus[m]||function(){window.privatus.q.push([m,Array.prototype.slice.call(arguments)])}})";

/// Text that shows a page already has the tracker.
pub const MARKER: &str = "/js/pa.js";

/// The script tag for one page. Get one with [`Config::tag`].
///
/// Every setting can be changed for this page only, and page props set here
/// are merged over the configured ones. A [`Value::Null`] (or `None`) removes
/// one.
///
/// ```
/// use privatus_analytics::{props, Config};
///
/// let config = Config::new().site_id("pa_YOURSITEID").enabled(true);
/// let html = config
///     .tag()
///     .modules(["engage", "auto"])
///     .props(props! { "plan" => "pro" })
///     .nonce("r4nd0m")
///     .to_html();
/// assert!(html.as_str().contains(r#"data-modules="engage,auto""#));
/// ```
///
/// `Tag` implements `Display`, so `format!("{tag}")` and `{{ tag }}` style
/// templates print the HTML without escaping it again.
#[derive(Clone, Debug)]
pub struct Tag<'a> {
    config: Cow<'a, Config>,
    nonce: Option<String>,
    props: Vec<(String, Value)>,
}

impl<'a> Tag<'a> {
    /// The tag for these settings.
    pub fn new(config: &'a Config) -> Self {
        Tag {
            config: Cow::Borrowed(config),
            nonce: None,
            props: Vec::new(),
        }
    }

    fn edit(mut self, change: impl FnOnce(Config) -> Config) -> Self {
        let config = std::mem::take(self.config.to_mut());
        *self.config.to_mut() = change(config);
        self
    }

    /// Uses another site id on this page.
    pub fn site_id(self, site_id: impl Into<String>) -> Self {
        let site_id = site_id.into();
        self.edit(|c| c.site_id(site_id))
    }

    /// Uses another script host on this page.
    pub fn script_host(self, host: impl Into<String>) -> Self {
        let host = host.into();
        self.edit(|c| c.script_host(host))
    }

    /// Loads these modules on this page.
    pub fn modules<I, S>(self, modules: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let modules: Vec<String> = modules.into_iter().map(Into::into).collect();
        self.edit(|c| c.modules(modules))
    }

    /// Uses these excluded paths on this page.
    pub fn excluded_paths<I, S>(self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let paths: Vec<String> = paths.into_iter().map(Into::into).collect();
        self.edit(|c| c.excluded_paths(paths))
    }

    /// Uses these domains on this page.
    pub fn domains<I, S>(self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let domains: Vec<String> = domains.into_iter().map(Into::into).collect();
        self.edit(|c| c.domains(domains))
    }

    /// Uses this single-page app mode on this page.
    pub fn spa(self, spa: Spa) -> Self {
        self.edit(|c| c.spa(spa))
    }

    /// Turns manual mode on or off on this page.
    pub fn manual(self, manual: bool) -> Self {
        self.edit(|c| c.manual(manual))
    }

    /// Forces the tag on or off on this page.
    pub fn enabled(self, enabled: bool) -> Self {
        self.edit(|c| c.enabled(enabled))
    }

    /// Adds or replaces one extra attribute on this page.
    pub fn extra_attribute(self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let (name, value) = (name.into(), value.into());
        self.edit(|c| c.extra_attribute(name, value))
    }

    /// Prints the queue stub on this page, or not.
    pub fn queue_stub(self, queue_stub: bool) -> Self {
        self.edit(|c| c.queue_stub(queue_stub))
    }

    /// The Content Security Policy nonce for the scripts.
    pub fn nonce(mut self, nonce: impl Into<String>) -> Self {
        let nonce = nonce.into();
        self.nonce = (!nonce.is_empty()).then_some(nonce);
        self
    }

    /// Merges page props over the configured ones. A [`Value::Null`] (or
    /// `None`) removes a prop. Can be called more than once.
    pub fn props<I, K, V>(mut self, props: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<Value>,
    {
        for (key, value) in props {
            merge(&mut self.props, normalize_key(key.as_ref()), value.into());
        }
        self
    }

    /// The settings this tag uses, with the per-page changes.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Whether anything prints: there is a site id and the tag is on.
    pub fn will_render(&self) -> bool {
        self.config.is_enabled() && self.config.resolved_site_id().is_some()
    }

    /// The configured props with the per-page props merged on top. Keys are
    /// cleaned. Null values are still in the list.
    pub fn page_props(&self) -> Vec<(String, Value)> {
        let mut merged = Vec::new();
        for (key, value) in self.config.configured_props().iter() {
            merge(&mut merged, normalize_key(key), value.clone());
        }
        for (key, value) in &self.props {
            merge(&mut merged, key.clone(), value.clone());
        }
        merged
    }

    /// The tracker tag's attributes in order. `None` is an attribute with no
    /// value (`defer`).
    pub fn attributes(&self) -> Vec<(String, Option<String>)> {
        let mut attrs: Vec<(String, Option<String>)> = vec![
            ("defer".into(), None),
            (
                "src".into(),
                Some(format!("{}{MARKER}", self.config.resolved_script_host())),
            ),
            (
                "data-site".into(),
                Some(self.config.resolved_site_id().unwrap_or_default()),
            ),
        ];
        // None keeps the tracker default. An empty modules list loads none.
        if let Some(modules) = self.config.modules_list() {
            attrs.push(("data-modules".into(), Some(list(modules).join(","))));
        }
        let excluded = list(self.config.excluded_paths_list());
        if !excluded.is_empty() {
            attrs.push(("data-exclude".into(), Some(excluded.join(","))));
        }
        if let Some(domains) = self.config.domains_list() {
            let domains = list(domains);
            if !domains.is_empty() {
                attrs.push(("data-domains".into(), Some(domains.join(","))));
            }
        }
        if self.config.spa_mode() != Spa::Auto {
            attrs.push(("data-spa".into(), Some(self.config.spa_mode().to_string())));
        }
        if self.config.is_manual() {
            attrs.push(("data-manual".into(), Some("true".into())));
        }
        for (name, value) in self.config.extra_attributes_list() {
            set_attr(&mut attrs, name.clone(), Some(value.clone()));
        }
        if let Some(nonce) = &self.nonce {
            set_attr(&mut attrs, "nonce".into(), Some(nonce.clone()));
        }
        attrs
    }

    /// The HTML: the queue stub (when on), the page props script (when there
    /// are props), then the tracker tag, one per line. Empty when the tag is
    /// off or there is no site id.
    pub fn to_html(&self) -> SafeHtml {
        if !self.will_render() {
            return SafeHtml::default();
        }
        let props_json = page_props_json(self.page_props());
        let mut parts = Vec::new();
        if self.config.wants_queue_stub() {
            parts.push(self.inline_script(QUEUE_STUB));
        }
        let mut attrs = self.attributes();
        if let Some(json) = &props_json {
            parts.push(self.inline_script(&format!(
                "window.{PROPS_FUNCTION}=function(){{return {json}}}"
            )));
            set_attr(&mut attrs, "data-props".into(), Some(PROPS_FUNCTION.into()));
        }
        let rendered: Vec<String> = attrs
            .iter()
            .map(|(name, value)| match value {
                None => name.clone(),
                Some(value) => format!("{name}=\"{}\"", escape_html(value)),
            })
            .collect();
        parts.push(format!("<script {}></script>", rendered.join(" ")));
        SafeHtml::from_trusted(parts.join("\n"))
    }

    fn inline_script(&self, code: &str) -> String {
        match &self.nonce {
            Some(nonce) => format!("<script nonce=\"{}\">{code}</script>", escape_html(nonce)),
            None => format!("<script>{code}</script>"),
        }
    }
}

impl fmt::Display for Tag<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_html().as_str())
    }
}

impl From<Tag<'_>> for SafeHtml {
    fn from(tag: Tag<'_>) -> SafeHtml {
        tag.to_html()
    }
}

/// Puts `tag` just before the first `</head>` (any case) in `html`.
///
/// Returns `None` when the page has no `</head>` or already has the tracker
/// (it contains `/js/pa.js`). Use it to add the tag in a framework this crate
/// has no middleware for.
///
/// ```
/// use privatus_analytics::inject_tag;
///
/// let page = "<html><head><title>Hi</title></head><body></body></html>";
/// let out = inject_tag(page, "<script defer src=\"https://privatusstats.com/js/pa.js\"></script>").unwrap();
/// assert!(out.contains("pa.js\"></script>\n</head>"));
/// assert_eq!(inject_tag(&out, "<script></script>"), None);
/// ```
pub fn inject_tag(html: &str, tag: &str) -> Option<String> {
    if html.contains(MARKER) {
        return None;
    }
    let at = find_ignore_ascii_case(html, "</head>")?;
    let mut out = String::with_capacity(html.len() + tag.len() + 1);
    out.push_str(&html[..at]);
    out.push_str(tag);
    out.push('\n');
    out.push_str(&html[at..]);
    Some(out)
}

fn find_ignore_ascii_case(haystack: &str, needle: &str) -> Option<usize> {
    let hay = haystack.as_bytes();
    let needle = needle.as_bytes();
    if hay.len() < needle.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()].eq_ignore_ascii_case(needle))
}

/// Whether a response should get the tag: status 200, `text/html`, not
/// compressed, and not an Ajax (`X-Requested-With: XMLHttpRequest`) request.
#[allow(dead_code)]
pub(crate) fn should_inject(
    status: u16,
    content_type: Option<&str>,
    encoded: bool,
    xhr: Option<&str>,
) -> bool {
    status == 200
        && !encoded
        && content_type.is_some_and(|t| t.trim().to_ascii_lowercase().starts_with("text/html"))
        && !xhr.is_some_and(|x| x.trim().eq_ignore_ascii_case("XMLHttpRequest"))
}

fn list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}

fn set_attr(attrs: &mut Vec<(String, Option<String>)>, name: String, value: Option<String>) {
    match attrs.iter_mut().find(|(k, _)| *k == name) {
        Some(slot) => slot.1 = value,
        None => attrs.push((name, value)),
    }
}

fn merge(props: &mut Vec<(String, Value)>, key: String, value: Value) {
    match props.iter_mut().find(|(k, _)| *k == key) {
        Some(slot) => slot.1 = value,
        None => props.push((key, value)),
    }
}
