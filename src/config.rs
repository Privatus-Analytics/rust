use crate::tag::Tag;
use crate::value::{Props, Value};
use std::fmt;
use std::str::FromStr;

/// Where `/js/pa.js` is served when nothing else is set.
pub const DEFAULT_SCRIPT_HOST: &str = "https://privatusstats.com";

/// How the tracker follows page changes in a single-page app (the `data-spa`
/// attribute).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Spa {
    /// The tracker default. Prints no attribute.
    #[default]
    Auto,
    /// Follow the History API (`pushState`).
    History,
    /// Follow changes to the `#hash` part of the address.
    Hash,
    /// Do not follow page changes.
    Off,
}

impl Spa {
    /// The attribute value: `auto`, `history`, `hash` or `off`.
    pub fn as_str(self) -> &'static str {
        match self {
            Spa::Auto => "auto",
            Spa::History => "history",
            Spa::Hash => "hash",
            Spa::Off => "off",
        }
    }
}

impl fmt::Display for Spa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The error for an unknown single-page app mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownSpaMode(pub String);

impl fmt::Display for UnknownSpaMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown spa mode {:?}: use auto, history, hash or off",
            self.0
        )
    }
}

impl std::error::Error for UnknownSpaMode {}

impl FromStr for Spa {
    type Err = UnknownSpaMode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Spa::Auto),
            "history" => Ok(Spa::History),
            "hash" => Ok(Spa::Hash),
            "off" => Ok(Spa::Off),
            _ => Err(UnknownSpaMode(s.to_owned())),
        }
    }
}

/// A Content Security Policy nonce for the current request.
///
/// Your own CSP middleware puts it in the request extensions (or a handler
/// puts it in the response extensions). The axum and actix-web middleware
/// read it from there and add it to the scripts they print.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CspNonce(pub String);

/// Settings for the tracking script.
///
/// Every setting is optional except the site id. Build it once at start up
/// and share it, for example in your app state.
///
/// ```
/// use privatus_analytics::{props, Config, Spa};
///
/// let config = Config::new()
///     .site_id("pa_YOURSITEID")
///     .modules(["engage", "auto"])
///     .excluded_paths(["/admin/*", "/preview/*"])
///     .spa(Spa::History)
///     .props(props! { "app_version" => "2.4" });
/// # let _ = config;
/// ```
///
/// ## When the tag prints
///
/// The tag prints nothing without a site id. When [`Config::enabled`] is not
/// set, the tag is on in release builds and off in debug builds. The check is
/// `cfg!(not(debug_assertions))`, so `cargo run` and `cargo test` print
/// nothing, and `cargo build --release` does. Set `enabled(true)` or
/// `enabled(false)` to choose yourself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    site_id: Option<String>,
    script_host: Option<String>,
    modules: Option<Vec<String>>,
    excluded_paths: Vec<String>,
    domains: Option<Vec<String>>,
    spa: Spa,
    manual: bool,
    enabled: Option<bool>,
    auto_inject: Option<bool>,
    extra_attributes: Vec<(String, String)>,
    props: Props,
    queue_stub: bool,
}

impl Config {
    /// Settings with every default. The site id then comes from the
    /// `PRIVATUS_SITE_ID` environment variable.
    pub fn new() -> Self {
        Config::default()
    }

    /// The site id, which starts with `pa_`. You find it under Site settings,
    /// Tracking. Without it, falls back to `PRIVATUS_SITE_ID`.
    pub fn site_id(mut self, site_id: impl Into<String>) -> Self {
        self.site_id = Some(site_id.into());
        self
    }

    /// Where `/js/pa.js` is served. Falls back to `PRIVATUS_SCRIPT_HOST`, then
    /// `https://privatusstats.com`. A trailing slash is removed. Change it
    /// only if you host your own copy.
    pub fn script_host(mut self, host: impl Into<String>) -> Self {
        self.script_host = Some(host.into());
        self
    }

    /// Tracker modules to load (`data-modules`), for example
    /// `["engage", "auto"]`. Setting it replaces the tracker default, so keep
    /// `engage` for time on page and scroll depth. An empty list loads none.
    pub fn modules<I, S>(mut self, modules: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.modules = Some(modules.into_iter().map(Into::into).collect());
        self
    }

    /// Goes back to the tracker's default modules.
    pub fn default_modules(mut self) -> Self {
        self.modules = None;
        self
    }

    /// Path globs the tracker never counts (`data-exclude`), for example
    /// `["/admin/*"]`.
    pub fn excluded_paths<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.excluded_paths = paths.into_iter().map(Into::into).collect();
        self
    }

    /// Count only these hostnames (`data-domains`). Not set counts every host.
    pub fn domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.domains = Some(domains.into_iter().map(Into::into).collect());
        self
    }

    /// Single-page app mode (`data-spa`). Only modes other than
    /// [`Spa::Auto`] are printed.
    pub fn spa(mut self, spa: Spa) -> Self {
        self.spa = spa;
        self
    }

    /// `true` turns off the automatic pageview (`data-manual="true"`).
    pub fn manual(mut self, manual: bool) -> Self {
        self.manual = manual;
        self
    }

    /// `true` or `false` forces the tag on or off. Not set means on in release
    /// builds and off in debug builds.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    /// Whether the axum and actix-web middleware add the tag. Adding the
    /// middleware turns it on, so this defaults to `true`. Set `false` to
    /// keep the middleware in place but switched off.
    pub fn auto_inject(mut self, auto_inject: bool) -> Self {
        self.auto_inject = Some(auto_inject);
        self
    }

    /// Adds any other attribute to the tracker tag, for example
    /// `("data-mask", "/orders/*->/orders/:id")`. A name set twice keeps the
    /// later value.
    pub fn extra_attribute(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        set_pair(&mut self.extra_attributes, name.into(), value.into());
        self
    }

    /// Page props sent with every pageview and event. For props that change
    /// per request, see the middleware's `props_fn` or [`Tag::props`].
    ///
    /// Never put names, emails, user ids or anything else that points to one
    /// person here.
    pub fn props<I, K, V>(mut self, props: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<Value>,
    {
        self.props = props.into_iter().collect();
        self
    }

    /// `true` prints the tracker's queue stub first, so `privatus.track`
    /// calls made before `pa.js` loads are kept and sent later.
    pub fn queue_stub(mut self, queue_stub: bool) -> Self {
        self.queue_stub = queue_stub;
        self
    }

    /// The tag for these settings. Change any setting for one page on the
    /// returned [`Tag`].
    pub fn tag(&self) -> Tag<'_> {
        Tag::new(self)
    }

    /// The site id in use: the configured one, else `PRIVATUS_SITE_ID`, else
    /// `None`.
    pub fn resolved_site_id(&self) -> Option<String> {
        let configured = self.site_id.as_deref().unwrap_or("").trim();
        if !configured.is_empty() {
            return Some(configured.to_owned());
        }
        let env = std::env::var("PRIVATUS_SITE_ID").unwrap_or_default();
        let env = env.trim();
        (!env.is_empty()).then(|| env.to_owned())
    }

    /// The script host in use, without a trailing slash.
    pub fn resolved_script_host(&self) -> String {
        let mut host = self.script_host.as_deref().unwrap_or("").trim().to_owned();
        if host.is_empty() {
            host = std::env::var("PRIVATUS_SCRIPT_HOST")
                .unwrap_or_default()
                .trim()
                .to_owned();
        }
        if host.is_empty() {
            host = DEFAULT_SCRIPT_HOST.to_owned();
        }
        if host.ends_with('/') {
            host.pop();
        }
        host
    }

    /// Whether the tag is on: the `enabled` setting when set, else
    /// `cfg!(not(debug_assertions))` (on in release builds).
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(cfg!(not(debug_assertions)))
    }

    /// Whether the middleware should add the tag. Defaults to `true`.
    pub fn is_auto_inject(&self) -> bool {
        self.auto_inject.unwrap_or(true)
    }

    pub(crate) fn modules_list(&self) -> Option<&[String]> {
        self.modules.as_deref()
    }

    pub(crate) fn excluded_paths_list(&self) -> &[String] {
        &self.excluded_paths
    }

    pub(crate) fn domains_list(&self) -> Option<&[String]> {
        self.domains.as_deref()
    }

    pub(crate) fn spa_mode(&self) -> Spa {
        self.spa
    }

    pub(crate) fn is_manual(&self) -> bool {
        self.manual
    }

    pub(crate) fn extra_attributes_list(&self) -> &[(String, String)] {
        &self.extra_attributes
    }

    pub(crate) fn configured_props(&self) -> &Props {
        &self.props
    }

    pub(crate) fn wants_queue_stub(&self) -> bool {
        self.queue_stub
    }
}

pub(crate) fn set_pair(pairs: &mut Vec<(String, String)>, name: String, value: String) {
    match pairs.iter_mut().find(|(k, _)| *k == name) {
        Some(slot) => slot.1 = value,
        None => pairs.push((name, value)),
    }
}
