use privatus_analytics::{props, Config, Spa, QUEUE_STUB};

fn config() -> Config {
    Config::new().site_id("pa_TEST123456").enabled(true)
}

const BASIC: &str =
    r#"<script defer src="https://privatusstats.com/js/pa.js" data-site="pa_TEST123456"></script>"#;

#[test]
fn prints_the_basic_snippet() {
    assert_eq!(config().tag().to_html().as_str(), BASIC);
    assert_eq!(config().tag().to_string(), BASIC);
    assert_eq!(privatus_analytics::tag(&config()).to_string(), BASIC);
}

#[test]
fn prints_nothing_without_a_site_id() {
    assert_eq!(
        Config::new().enabled(true).site_id("  ").tag().to_string(),
        ""
    );
}

#[test]
fn prints_nothing_when_disabled() {
    assert_eq!(config().enabled(false).tag().to_string(), "");
}

#[test]
fn enabled_defaults_to_release_builds() {
    let config = Config::new().site_id("pa_TEST123456");
    assert_eq!(config.is_enabled(), cfg!(not(debug_assertions)));
    assert_eq!(config.tag().will_render(), cfg!(not(debug_assertions)));
}

#[test]
fn script_host_drops_a_trailing_slash() {
    let html = config()
        .script_host("https://stats.example.com/")
        .tag()
        .to_string();
    assert!(
        html.contains(r#"src="https://stats.example.com/js/pa.js""#),
        "{html}"
    );
}

#[test]
fn adds_modules_and_exclusions() {
    let html = config()
        .modules(["engage", "auto"])
        .excluded_paths(["/admin/*", "/preview/*"])
        .tag()
        .to_string();
    assert!(html.contains(r#"data-modules="engage,auto""#));
    assert!(html.contains(r#"data-exclude="/admin/*,/preview/*""#));
}

#[test]
fn an_empty_modules_list_loads_none() {
    let html = config().modules(Vec::<String>::new()).tag().to_string();
    assert!(html.contains(r#"data-modules="""#));
    assert!(!config().tag().to_string().contains("data-modules"));
}

#[test]
fn adds_domains_spa_mode_and_manual_mode() {
    let html = config()
        .domains(["example.com, www.example.com"])
        .spa(Spa::Hash)
        .manual(true)
        .tag()
        .to_string();
    assert!(html.contains(r#"data-domains="example.com,www.example.com""#));
    assert!(html.contains(r#"data-spa="hash""#));
    assert!(html.contains(r#"data-manual="true""#));
}

#[test]
fn the_auto_spa_mode_prints_nothing() {
    assert!(!config()
        .spa(Spa::Auto)
        .tag()
        .to_string()
        .contains("data-spa"));
    assert_eq!("history".parse::<Spa>(), Ok(Spa::History));
    assert!("bogus".parse::<Spa>().is_err());
}

#[test]
fn empty_domains_print_nothing() {
    assert!(!config()
        .domains([" "])
        .tag()
        .to_string()
        .contains("data-domains"));
}

#[test]
fn escapes_attribute_values() {
    let html = config().excluded_paths([r#"/a"b"#]).tag().to_string();
    assert!(html.contains(r#"data-exclude="/a&quot;b""#));
}

#[test]
fn per_call_options_override_the_configuration() {
    let config = config().modules(["engage"]);
    let html = config.tag().modules(["vitals"]).to_string();
    assert!(html.contains(r#"data-modules="vitals""#));
    assert!(config
        .tag()
        .to_string()
        .contains(r#"data-modules="engage""#));
    assert_eq!(config.tag().enabled(false).to_string(), "");
    assert!(config
        .tag()
        .site_id("pa_OTHER")
        .to_string()
        .contains(r#"data-site="pa_OTHER""#));
    assert!(config
        .tag()
        .spa(Spa::Off)
        .to_string()
        .contains(r#"data-spa="off""#));
}

#[test]
fn adds_extra_attributes_and_a_nonce() {
    let html = config()
        .extra_attribute("data-mask", "/orders/*->/orders/:id")
        .tag()
        .nonce("abc123")
        .to_string();
    assert!(html.contains(r#"data-mask="/orders/*-&gt;/orders/:id""#));
    assert!(html.contains(r#"nonce="abc123""#));
}

#[test]
fn page_props_print_an_inline_script_before_the_tag() {
    let html = config()
        .tag()
        .props(props! { "plan" => "pro", "role" => "admin" })
        .to_string();
    let expected = concat!(
        r#"<script>window.privatusPageProps=function(){return {"plan":"pro","role":"admin"}}</script>"#,
        "\n",
        r#"<script defer src="https://privatusstats.com/js/pa.js" data-site="pa_TEST123456" data-props="privatusPageProps"></script>"#
    );
    assert_eq!(html, expected);
}

#[test]
fn the_props_script_gets_the_nonce() {
    let html = config()
        .tag()
        .props(props! { "plan" => "pro" })
        .nonce("abc")
        .to_string();
    assert!(html.starts_with(r#"<script nonce="abc">window.privatusPageProps="#));
}

#[test]
fn empty_props_print_no_script() {
    let html = config()
        .tag()
        .props(props! { "plan" => None::<&str> })
        .to_string();
    assert!(!html.contains("privatusPageProps"));
    assert_eq!(html, BASIC);
}

#[test]
fn configured_props_are_used() {
    let html = config().props(props! { "plan" => "pro" }).tag().to_string();
    assert!(html.contains(r#"{"plan":"pro"}"#));
}

#[test]
fn per_call_props_merge_over_configured_props_and_null_removes_one() {
    let config = config().props(props! { "plan" => "pro", "role" => "admin" });
    let html = config
        .tag()
        .props(props! { "Role" => None::<&str>, "tier" => "gold" })
        .to_string();
    assert!(html.contains(r#"{"plan":"pro","tier":"gold"}"#), "{html}");
}

#[test]
fn queue_stub_prints_the_official_stub_first() {
    let html = config()
        .queue_stub(true)
        .tag()
        .nonce("abc")
        .props(props! { "plan" => "pro" })
        .to_string();
    let parts: Vec<&str> = html.split('\n').collect();
    assert_eq!(
        parts[0],
        format!(r#"<script nonce="abc">{QUEUE_STUB}</script>"#)
    );
    assert!(parts[1].contains("privatusPageProps"));
    assert!(parts[2].contains("/js/pa.js"));
}

#[test]
fn the_queue_stub_is_the_official_one() {
    assert_eq!(
        QUEUE_STUB,
        "window.privatus=window.privatus||{q:[]};['track','pageview','props','optOut','optIn','ready'].forEach(function(m){window.privatus[m]=window.privatus[m]||function(){window.privatus.q.push([m,Array.prototype.slice.call(arguments)])}})"
    );
}

#[test]
fn queue_stub_can_be_switched_on_per_call() {
    assert!(config().tag().queue_stub(true).to_string().contains("q:[]"));
    assert!(!config().tag().to_string().contains("q:[]"));
}

#[test]
fn inject_tag_puts_the_tag_before_head() {
    let page = "<html><HEAD><title>x</title></HEAD><body></body></html>";
    let out = privatus_analytics::inject_tag(page, BASIC).unwrap();
    assert_eq!(
        out,
        format!("<html><HEAD><title>x</title>{BASIC}\n</HEAD><body></body></html>")
    );
    assert_eq!(privatus_analytics::inject_tag(&out, BASIC), None);
    assert_eq!(
        privatus_analytics::inject_tag("<p>no head</p>", BASIC),
        None
    );
}

// When the tag is off, only the queue stub prints (if it is on), so
// privatus.track calls in your own JavaScript never fail.

#[test]
fn off_with_the_queue_stub_prints_only_the_stub_with_the_nonce() {
    let html = config()
        .enabled(false)
        .queue_stub(true)
        .props(props! { "plan" => "pro" })
        .tag()
        .nonce("abc")
        .to_string();
    assert_eq!(
        html,
        format!(r#"<script nonce="abc">{QUEUE_STUB}</script>"#)
    );
}

#[test]
fn off_without_the_queue_stub_prints_nothing() {
    let html = config()
        .enabled(false)
        .queue_stub(false)
        .tag()
        .props(props! { "plan" => "pro" })
        .nonce("abc")
        .to_string();
    assert_eq!(html, "");
}

#[test]
fn no_site_id_with_the_queue_stub_prints_only_the_stub() {
    let html = Config::new()
        .enabled(true)
        .site_id(" ")
        .tag()
        .queue_stub(true)
        .props(props! { "plan" => "pro" })
        .to_string();
    assert_eq!(html, format!("<script>{QUEUE_STUB}</script>"));
}

#[test]
fn will_render_is_about_the_tracker_not_the_stub() {
    assert!(!config().enabled(false).queue_stub(true).tag().will_render());
}
