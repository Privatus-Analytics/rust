use privatus_analytics::{
    click_attr, click_data, event_attrs, event_data, props, valid_click, Props,
};

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn get(attrs: &[(String, String)], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
}

#[test]
fn an_event_becomes_data_attributes() {
    assert_eq!(
        event_data(
            "Export CSV",
            props! { "format" => "csv", "source" => "toolbar" }
        ),
        pairs(&[
            ("data-privatus-event", "Export CSV"),
            ("data-privatus-prop-format", "csv"),
            ("data-privatus-prop-source", "toolbar"),
        ])
    );
    assert_eq!(
        event_attrs("Export CSV", [("format", "csv"), ("source", "toolbar")]).as_str(),
        r#"data-privatus-event="Export CSV" data-privatus-prop-format="csv" data-privatus-prop-source="toolbar""#
    );
}

#[test]
fn the_event_name_is_trimmed_and_cut_to_120_characters() {
    let attrs = event_data("  Export CSV  ", Props::new());
    assert_eq!(get(&attrs, "data-privatus-event").unwrap(), "Export CSV");
    let long = "a".repeat(200);
    let attrs = event_data(&long, Props::new());
    assert_eq!(get(&attrs, "data-privatus-event").unwrap(), "a".repeat(120));
    let accents = "é".repeat(130);
    assert_eq!(
        get(&event_data(&accents, Props::new()), "data-privatus-event")
            .unwrap()
            .chars()
            .count(),
        120
    );
}

#[test]
fn an_empty_event_name_gives_nothing() {
    assert!(event_data("   ", props! { "format" => "csv" }).is_empty());
    assert!(event_attrs("", Props::new()).is_empty());
}

#[test]
fn prop_keys_are_lowercased_cleaned_and_cut_to_64_characters() {
    let long_key = "K".repeat(80);
    let attrs = event_data(
        "Saved",
        props! { "Report Type" => "a", "x-y" => "b", long_key => "c" },
    );
    assert_eq!(get(&attrs, "data-privatus-prop-report_type").unwrap(), "a");
    assert_eq!(get(&attrs, "data-privatus-prop-x_y").unwrap(), "b");
    assert_eq!(
        get(&attrs, &format!("data-privatus-prop-{}", "k".repeat(64))).unwrap(),
        "c"
    );
}

#[test]
fn values_become_strings_and_none_values_are_dropped() {
    let attrs = event_data(
        "Saved",
        props! {
            "on" => true, "off" => false, "count" => 3, "ratio" => 1.5,
            "tags" => vec!["a", "b"], "gone" => None::<i32>,
        },
    );
    let values: Vec<String> = ["on", "off", "count", "ratio", "tags"]
        .iter()
        .map(|k| get(&attrs, &format!("data-privatus-prop-{k}")).unwrap())
        .collect();
    assert_eq!(values, ["true", "false", "3", "1.5", "a,b"]);
    assert!(get(&attrs, "data-privatus-prop-gone").is_none());
}

#[test]
fn values_are_cut_to_500_characters_and_at_most_30_props_are_kept() {
    let many: Props = (1..=40).map(|i| (format!("p{i}"), "v")).collect();
    assert_eq!(event_data("Saved", many).len(), 31);
    let attrs = event_data("Saved", props! { "long" => "x".repeat(900) });
    assert_eq!(get(&attrs, "data-privatus-prop-long").unwrap().len(), 500);
}

#[test]
fn null_values_do_not_count_toward_the_limit() {
    let mut many: Props = (1..=5).map(|i| (format!("n{i}"), None::<&str>)).collect();
    many.extend((1..=30).map(|i| (format!("p{i}"), "v")));
    assert_eq!(event_data("Saved", many).len(), 31);
}

#[test]
fn html_escapes_every_value() {
    assert_eq!(
        event_attrs(r#"Say "hi" <now>"#, props! { "note" => "a&b" }).as_str(),
        r#"data-privatus-event="Say &quot;hi&quot; &lt;now&gt;" data-privatus-prop-note="a&amp;b""#
    );
}

#[test]
fn borrowed_props_work_too() {
    let props = props! { "format" => "csv" };
    assert_eq!(
        event_attrs("Export", &props),
        event_attrs("Export", props.clone())
    );
}

#[test]
fn a_valid_click_name_becomes_a_data_attribute() {
    assert_eq!(
        click_data("pricing.cta"),
        pairs(&[("data-privatus-click", "pricing.cta")])
    );
    assert_eq!(
        click_attr("a-b_c:d.1").as_str(),
        r#"data-privatus-click="a-b_c:d.1""#
    );
    assert!(valid_click(&"x".repeat(100)));
}

#[test]
fn an_invalid_click_name_gives_nothing() {
    for name in ["", "has space", &"x".repeat(101), "<b>", "é"] {
        assert!(click_data(name).is_empty(), "{name}");
        assert!(click_attr(name).is_empty());
        assert!(!valid_click(name));
    }
}
