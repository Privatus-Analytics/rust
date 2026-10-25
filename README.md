# Privatus Analytics for Rust

[![crates.io](https://img.shields.io/crates/v/privatus-analytics.svg)](https://crates.io/crates/privatus-analytics)
[![docs.rs](https://docs.rs/privatus-analytics/badge.svg)](https://docs.rs/privatus-analytics)

Adds the cookieless [Privatus Analytics](https://privatusanalytics.com)
tracking script to your Rust web app. Print it in your templates, or let an
optional axum or actix-web middleware put it in the `<head>` for you.

Privatus Analytics counts visits, sources, pages and conversions without
cookies, local storage or fingerprinting, so analytics does not need a consent
banner by itself. Check with your own counsel for your country.

The crate only prints HTML for the browser. It never sends events or any
other request from your server, sets no cookies, and never reads IP addresses,
browser details (user agents) or any visitor id. The core has no
dependencies. You need a Privatus Analytics account to see reports, and there
is a free plan. Sign up at https://privatusanalytics.com.

## Requirements

* Rust 1.75 or later for the core. The optional features need what their
  framework needs (for example actix-web 4 and askama 0.16 need a newer Rust).
* Works with any framework and template engine. There is built-in support
  for axum, actix-web, askama and maud.
* A Privatus Analytics site id (it starts with `pa_`). You find it under
  Site settings, Tracking.

## Install

```sh
cargo add privatus-analytics
```

Turn on the features you want:

```sh
cargo add privatus-analytics --features axum,askama
```

| Feature | What it adds |
|---|---|
| `axum` | `privatus_analytics::axum::PrivatusLayer`, a tower layer that adds the tag to HTML pages |
| `actix` | `privatus_analytics::actix::PrivatusMiddleware`, the same for actix-web |
| `askama` | askama prints the helpers as HTML, without escaping them again |
| `maud` | The helpers work inside `html!`, plus `privatus_analytics::maud::element` |

For the full setup guide and every tracker option, see the [Privatus
Analytics docs](https://privatusanalytics.com/docs). The Rust API is on
[docs.rs](https://docs.rs/privatus-analytics).

Set your site id with an environment variable:

```sh
PRIVATUS_SITE_ID=pa_YOURSITEID
```

or in code:

```rust
use privatus_analytics::Config;

let analytics = Config::new().site_id("pa_YOURSITEID");
```

Build the `Config` once when your app starts and share it, for example in
your app state.

## Add the tag

Print the tag in the `<head>` of your layout. `config.tag()` returns a `Tag`.
It prints with `{}` and works in any template engine:

```rust
use privatus_analytics::Config;

let analytics = Config::new().site_id("pa_YOURSITEID");
let page = format!("<head><title>My app</title>{}</head>", analytics.tag());
```

With askama (feature `askama`), put the tag in a field and print it. askama
does not escape it:

```rust
#[derive(askama::Template)]
#[template(path = "layout.html")]
struct Layout<'a> {
    analytics: privatus_analytics::Tag<'a>,
}
```

```html
<head>
  <title>My app</title>
  {{ analytics }}
</head>
```

With maud (feature `maud`), splice it in: `head { (analytics.tag()) }`.

In a release build it prints:

```html
<script defer src="https://privatusstats.com/js/pa.js" data-site="pa_YOURSITEID"></script>
```

The tag prints nothing when there is no site id, and nothing in debug builds
(see below), except the queue stub when you turn it on. If your app uses a Content Security Policy nonce, pass it with
`analytics.tag().nonce(nonce)`.

### Or let the middleware add it

If you would rather not touch your templates, add the middleware.

axum (feature `axum`):

```rust
use axum::Router;
use privatus_analytics::{axum::PrivatusLayer, Config};

let app: Router = Router::new()
    // .route(...)
    .layer(PrivatusLayer::new(Config::new().site_id("pa_YOURSITEID")));
```

actix-web (feature `actix`):

```rust
use actix_web::App;
use privatus_analytics::{actix::PrivatusMiddleware, Config};

let app = App::new()
    .wrap(PrivatusMiddleware::new(Config::new().site_id("pa_YOURSITEID")));
```

It adds the tag just before `</head>` in uncompressed `text/html` responses
with status 200. It skips Ajax requests (`X-Requested-With: XMLHttpRequest`),
pages without a `<head>`, and pages that already have the tag. It fixes the
`Content-Length` header. Responses that carry a `Content-Encoding` header are
left alone, so add the middleware inside your compression layer (in axum,
call `.layer(PrivatusLayer::new(..))` before `.layer(CompressionLayer::new())`;
in actix-web, call `.wrap(PrivatusMiddleware::new(..))` before
`.wrap(Compress::default())`), or print the tag yourself.

For a Content Security Policy nonce, put a `privatus_analytics::CspNonce` in
the request extensions from your CSP middleware, or in the response
extensions from a handler. The middleware adds it to the scripts it prints.

## Configuration

Every setting is optional except the site id.

```rust
use privatus_analytics::{props, Config, Spa};

let analytics = Config::new()
    .site_id("pa_YOURSITEID")                    // or PRIVATUS_SITE_ID
    .script_host("https://privatusstats.com")    // or PRIVATUS_SCRIPT_HOST
    .modules(["engage", "auto", "vitals", "clicks", "ask"])
    .excluded_paths(["/admin/*", "/preview/*"])
    .domains(["example.com", "www.example.com"])
    .spa(Spa::Auto)                              // Auto, History, Hash or Off
    .manual(false)                               // true turns off the automatic pageview
    .enabled(true)                               // leave out to use the build type
    .auto_inject(true)
    .extra_attribute("data-mask", "/orders/*->/orders/:id")
    .props(props! { "app_version" => "2.4" })
    .queue_stub(false);
```

| Setting | Default | What it does |
|---|---|---|
| `site_id` | `PRIVATUS_SITE_ID` | The `data-site` attribute. Nothing prints without it |
| `script_host` | `https://privatusstats.com` | Where `/js/pa.js` is served. Falls back to `PRIVATUS_SCRIPT_HOST`. Change it only if you host your own copy |
| `modules` | tracker default (`engage`) | The `data-modules` attribute. Setting it replaces the default, so keep `engage` for time on page and scroll depth. An empty list loads none |
| `excluded_paths` | none | The `data-exclude` attribute. Path globs the tracker never counts |
| `domains` | all hosts | The `data-domains` attribute. Count only these hostnames |
| `spa` | `Spa::Auto` | The `data-spa` attribute. `Auto` prints nothing |
| `manual` | `false` | The `data-manual` attribute |
| `enabled` | not set | `true` or `false` forces the tag on or off. Not set means on in release builds only |
| `auto_inject` | `true` | Whether the axum and actix-web middleware add the tag. Adding the middleware turns it on. `false` keeps it in place but off |
| `extra_attribute` | none | Any other `data-*` attribute |
| `props` | none | Page props sent with every pageview and event. See below |
| `queue_stub` | `false` | Prints a small script first, so `privatus.track` calls made before the tracker loads are kept and sent later |

Each tracker attribute is explained in the [script attributes
reference](https://privatusanalytics.com/docs/tracker/attributes), and the
modules in [Modules](https://privatusanalytics.com/docs/tracker/modules).

You can also change any setting for one page. The `Tag` has the same
methods:

```rust
let tag = analytics.tag().modules(["engage", "auto"]).excluded_paths(["/preview/*"]);
```

### Development and test

When `enabled` is not set, the tag prints only in release builds. The check
is `cfg!(not(debug_assertions))`: `cargo run` and `cargo test` print nothing,
and `cargo build --release` does. So your own clicking around never reaches
your reports. If you turn on `debug-assertions` in your release profile, set
`enabled(true)` there yourself.

To see the tag while you build, set `.enabled(true)`. The tracker also
ignores `localhost` unless you pass `data-allow-local` or `data-debug` (see
[Debug mode](https://privatusanalytics.com/docs/tracker/debug)):

```rust
let analytics = analytics.enabled(true).extra_attribute("data-debug", "true");
```

If `queue_stub` is on, the tag still prints the small queue stub (and nothing
else), so `privatus.track` calls in your own JavaScript keep working. They
wait in a queue and are never sent. The middleware adds nothing when the tag
is off.

### Leaving people out

Use `data-exclude` through `excluded_paths` for admin areas. To leave out
signed-in staff, print the tag only for other people:

```rust
let tag = if user.is_admin { None } else { Some(analytics.tag()) };
```

or switch it off for one page with `analytics.tag().enabled(false)`.

## Product analytics: see which features people use

Privatus can show which buttons, forms and features people really use. You
add a few attributes to your HTML. The tracker reads them in the browser and
sends the events. Your server still sends nothing.

### Name events well

Use a short "Noun verb" name, like `Report exported` or `Invite sent`. Put the
details in props, not in the name. So write `Report exported` with
`"format" => "csv"`, not `Report exported as CSV`. Then all exports add up in
one place, and you can split them by format.

### Events on buttons, links and forms

`event_attrs` returns the attributes to print inside a tag you write
yourself:

```rust
use privatus_analytics::{event_attrs, props};

let export = event_attrs("Report exported", props! { "format" => "csv", "source" => "toolbar" });
let button = format!("<button {export}>Export</button>");
```

In askama, put it in a field and write `<button {{ export }}>Export</button>`.
It prints:

```html
<button data-privatus-event="Report exported" data-privatus-prop-format="csv" data-privatus-prop-source="toolbar">Export</button>
```

The props can be the `props!` macro, a `Props`, or any list of pairs, like
`[("format", "csv")]`. Values can be text, numbers, `true` or `false`,
`Option` (`None` is left out) or lists.

maud does not let you add attributes from a string, so use `element`
(feature `maud`):

```rust
use maud::html;
use privatus_analytics::{event_attrs, maud::element, props};

let markup = html! {
    (element("button", &event_attrs("Report exported", props! { "format" => "csv" }), "Export"))
};
```

When you would rather set attributes one by one (for example with a builder
that escapes for you), `event_data` returns the same attributes as a list of
`(name, value)` pairs that are not escaped yet.

On a form, the event is sent when the form is submitted:

```rust
let form = format!(
    r#"<form action="/newsletter" method="post" {}>"#,
    event_attrs("Newsletter joined", props! { "list" => "weekly" })
);
```

The rules:

* The name is trimmed and cut to 120 characters. An empty name prints nothing.
* Prop keys are made lowercase. Anything that is not a letter, a digit or `_`
  becomes `_`, so `"Report Type"` becomes `report_type`. Keys are cut to 64
  characters.
* At most 30 props. Values become text: `true`, `false`, numbers as written,
  lists joined with commas. `None` values are left out. Values are cut to 500
  characters.
* Everything is escaped for HTML.
* The attributes print in every build, even when the tag is off. They do
  nothing without the tracker, so your pages look the same in development and
  production.

### Click maps

A click map shows where people click on a page. Give the important parts a
name so they stay easy to find, even when the page changes:

```rust
use privatus_analytics::click_attr;

let link = format!(r#"<a href="/pricing" {}>See plans</a>"#, click_attr("pricing.cta"));
```

A name may use letters, digits and `_ . : -`, up to 100 characters. Anything
else prints nothing, and in debug builds the crate writes a warning to
standard error. `valid_click` tells you if a name is valid, and `click_data`
returns the attribute as a pair.

### Page props: plan, role and other groups

Page props are sent with every pageview and event on the page. Use them to
split your reports by plan, role, app version, feature flag or account tier.

Set props for every page in the config:

```rust
let analytics = Config::new().props(props! { "app_version" => "2.4" });
```

Set or change props for one page on the tag. They are merged over the
configured ones, and `None` removes one:

```rust
let tag = analytics.tag().props(props! { "plan" => account.plan.as_str(), "role" => None::<&str> });
```

With the middleware, give it a function that runs on each request. In axum
it gets the request parts (path, headers and extensions, where your auth
middleware may have put the account). In actix-web it gets the
`HttpRequest`:

```rust
let layer = PrivatusLayer::new(analytics).props_fn(|parts| {
    let plan = parts.extensions.get::<Account>().map(|a| a.plan.clone());
    props! { "plan" => plan }
});
```

The function runs only when the tag is on. A handler or another middleware
can also put a `Props` in the request or response extensions. Those are
merged over the configured props and the function's props.

The tag then prints a small script before the tracker, with your Content
Security Policy nonce when you have one, and points the tracker at it:

```html
<script nonce="...">window.privatusPageProps=function(){return {"plan":"pro","role":"admin"}}</script>
<script defer src="https://privatusstats.com/js/pa.js" data-site="pa_YOURSITEID" data-props="privatusPageProps"></script>
```

Keys follow the same rules as event props. Numbers and true or false stay as
they are, and lists stay lists (up to 30 items). The props are escaped so they
are safe inside a script.

**Never put names, emails, user ids or anything that points to one person in
props.** Privatus counts visitors without any id. Put groups (like the plan or
the role), not people. The server also removes emails and long ids it finds.

### Track from your own JavaScript

For things that are not a click or a submit, call the tracker from your
JavaScript, as the [JavaScript API
docs](https://privatusanalytics.com/docs/tracker/javascript-api) explain:

```js
privatus.track("Feature used", { feature: "export" })
```

If your code may run before the tracker has loaded, switch on the queue stub.
It prints a tiny script before the tracker that keeps your calls in a list and
sends them once the tracker is ready:

```rust
let analytics = analytics.queue_stub(true);
```

or for one page: `analytics.tag().queue_stub(true)`.

The stub prints even when the tag is off (in debug builds, without a site id,
or with `.enabled(false)`). Only the stub prints then: no tracker and no page
props. So your `privatus.track` calls never fail; they wait in the queue and
are never sent.

### See the results

* [Funnels](https://privatusanalytics.com/docs/features/funnels): how many
  people go from one step to the next.
* [Click maps](https://privatusanalytics.com/docs/features/click-maps): where
  people click.
* [Goals](https://privatusanalytics.com/docs/features/goals): turn an event
  into a goal to count conversions.
* [Events](https://privatusanalytics.com/docs/events): every custom event and
  its props.

## Other frameworks

For a framework without a middleware here, `inject_tag(html, tag)` puts the
tag before `</head>` in a page you already have as a string. It returns
`None` when the page has no `</head>` or already has the tag.

## Privacy

The script sets no cookies and stores nothing in the browser. The event, click
and props helpers only print HTML attributes and a small script. Keep people
out of your props and event props: no names, emails or user ids. See the
[privacy documentation](https://privatusanalytics.com/docs/privacy) for what is
collected.

If you have a privacy policy, it should say which tools handle your visitors' data. You can paste this:

> This website uses [Privatus Analytics](https://privatusanalytics.com/privacy-analytics) to count visits. It
> does not use cookies and does not follow you from one website to another. Your IP address is used for a moment
> to count visits and is never stored.

Change it to fit your site. The [privacy policy generator](https://privatusanalytics.com/tools/privacy-policy-generator)
makes a longer version with your site's name in it.

## Development

The checks run in Docker so your local Rust does not matter:

```sh
./run-docker.sh
```

It runs rustfmt, Clippy, the tests (with and without features, in debug and
release builds), the docs build and `cargo package`.

## Contributing

Bug reports and pull requests are welcome at
https://github.com/Privatus-Analytics/rust.

## License

MIT. See [LICENSE](LICENSE). Privatus Analytics is a product of Two Phase LLC,
Wyoming, USA.
