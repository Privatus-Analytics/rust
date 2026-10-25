#[cfg(feature = "askama")]
mod askama_templates {
    use askama::Template;
    use privatus_analytics::{click_attr, event_attrs, props, Config, SafeHtml, Tag};

    #[derive(Template)]
    #[template(
        source = "<head>{{ tag }}</head><button {{ export }}>Export</button><a {{ cta }}>Plans</a><p>{{ text }}</p>",
        ext = "html"
    )]
    struct Page<'a> {
        tag: Tag<'a>,
        export: SafeHtml,
        cta: SafeHtml,
        text: &'a str,
    }

    #[test]
    fn askama_prints_the_helpers_without_escaping_them() {
        let config = Config::new().site_id("pa_TEST123456").enabled(true);
        let page = Page {
            tag: config.tag().props(props! { "plan" => "pro" }),
            export: event_attrs("Report exported", props! { "format" => "csv" }),
            cta: click_attr("pricing.cta"),
            text: "<b>",
        };
        let html = page.render().unwrap();
        assert!(
            html.contains(
                r#"<script>window.privatusPageProps=function(){return {"plan":"pro"}}</script>"#
            ),
            "{html}"
        );
        assert!(html.contains(
            r#"data-site="pa_TEST123456" data-props="privatusPageProps"></script></head>"#
        ));
        assert!(html.contains(
            r#"<button data-privatus-event="Report exported" data-privatus-prop-format="csv">"#
        ));
        assert!(html.contains(r#"<a data-privatus-click="pricing.cta">"#));
        assert!(
            html.contains("<p>&#60;b&#62;</p>") || html.contains("<p>&lt;b&gt;</p>"),
            "{html}"
        );
    }
}

#[cfg(feature = "maud")]
mod maud_templates {
    use maud::html;
    use privatus_analytics::maud::element;
    use privatus_analytics::{click_attr, event_attrs, props, Config, SafeHtml};

    #[test]
    fn maud_renders_the_tag_and_elements() {
        let config = Config::new().site_id("pa_TEST123456").enabled(true);
        let page = html! {
            head { (config.tag()) }
            (element("button", &event_attrs("Report exported", props! { "format" => "csv" }), html! { "Export" }))
            (element("a", &click_attr("pricing.cta"), "<Plans>"))
            (element("div", &SafeHtml::default(), "x"))
            (element("bad tag", &SafeHtml::default(), "y"))
        }
        .into_string();
        assert!(page.contains(r#"<head><script defer src="https://privatusstats.com/js/pa.js" data-site="pa_TEST123456"></script></head>"#), "{page}");
        assert!(page.contains(r#"<button data-privatus-event="Report exported" data-privatus-prop-format="csv">Export</button>"#));
        assert!(
            page.contains(r#"<a data-privatus-click="pricing.cta">&lt;Plans&gt;</a>"#),
            "{page}"
        );
        assert!(page.contains("<div>x</div>"));
        assert!(page.ends_with("</div>y"));
    }
}
