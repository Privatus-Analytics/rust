use privatus_analytics::{page_props_json, props, Props, Value};

#[test]
fn props_become_a_json_object() {
    assert_eq!(
        page_props_json(props! { "plan" => "pro", "role" => "admin" }).unwrap(),
        r#"{"plan":"pro","role":"admin"}"#
    );
}

#[test]
fn empty_props_give_none() {
    assert_eq!(page_props_json(Props::new()), None);
    assert_eq!(page_props_json(props! { "gone" => Value::Null }), None);
    assert_eq!(page_props_json(props! { "bad" => f64::NAN }), None);
}

#[test]
fn numbers_booleans_and_lists_keep_their_json_types() {
    let list = Value::List(vec!["a".into(), Value::Null, "b".into(), 3.into()]);
    assert_eq!(
        page_props_json(props! {
            "seats" => 5, "ratio" => 1.5, "beta" => true, "old" => false, "flags" => list,
            "big" => u64::MAX, "f" => 1.1_f32, "whole" => 3.0,
        })
        .unwrap(),
        r#"{"seats":5,"ratio":1.5,"beta":true,"old":false,"flags":["a","b",3],"big":18446744073709551615,"f":1.1,"whole":3.0}"#
    );
}

#[test]
fn keys_are_cleaned_like_event_props() {
    assert_eq!(
        page_props_json(props! { "App Version" => "2.1" }).unwrap(),
        r#"{"app_version":"2.1"}"#
    );
}

#[test]
fn limits_props_list_items_and_value_length() {
    let many: Props = (1..=40).map(|i| (format!("p{i}"), i)).collect();
    let json = page_props_json(many).unwrap();
    assert_eq!(json.matches(':').count(), 30);

    let items: Vec<String> = (1..=50).map(|i| i.to_string()).collect();
    let json = page_props_json(props! { "list" => items }).unwrap();
    assert_eq!(json.matches(',').count(), 29);

    let json = page_props_json(props! { "long" => "x".repeat(900) }).unwrap();
    assert_eq!(json.matches('x').count(), 500);
}

#[test]
fn the_json_is_safe_inside_a_script_tag() {
    let json = page_props_json(props! { "note" => "</script><b>&\u{2028}\u{2029}" }).unwrap();
    assert_eq!(
        json,
        r#"{"note":"\u003c/script\u003e\u003cb\u003e\u0026\u2028\u2029"}"#
    );
    assert!(!json.contains(['<', '>', '&', '\u{2028}', '\u{2029}']));
}

#[test]
fn quotes_backslashes_and_control_characters_are_escaped() {
    let json = page_props_json(props! { "q" => "a\"b\\c\nd\u{1}" }).unwrap();
    assert_eq!(json, r#"{"q":"a\"b\\c\nd\u0001"}"#);
}
