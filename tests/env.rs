// Runs in its own process, so setting environment variables here cannot race other tests.
use privatus_analytics::Config;

#[test]
fn site_id_and_script_host_fall_back_to_the_environment() {
    std::env::set_var("PRIVATUS_SITE_ID", " pa_FROMENV ");
    std::env::set_var("PRIVATUS_SCRIPT_HOST", "https://cdn.example.com/");
    let html = Config::new().enabled(true).tag().to_string();
    assert_eq!(
        html,
        r#"<script defer src="https://cdn.example.com/js/pa.js" data-site="pa_FROMENV"></script>"#
    );

    let configured = Config::new()
        .site_id("pa_SET")
        .script_host("https://x.example")
        .enabled(true);
    assert!(configured
        .tag()
        .to_string()
        .contains(r#"data-site="pa_SET""#));
    assert!(configured
        .tag()
        .to_string()
        .contains("https://x.example/js/pa.js"));

    std::env::remove_var("PRIVATUS_SITE_ID");
    std::env::remove_var("PRIVATUS_SCRIPT_HOST");
    assert_eq!(Config::new().enabled(true).tag().to_string(), "");
    assert_eq!(
        Config::new().resolved_script_host(),
        "https://privatusstats.com"
    );
}
