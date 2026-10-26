# Changelog

## 1.0.0 - 2026-10-25

- Initial release.
- `Config` with site id, script host, modules, excluded paths, domains,
  single-page app mode, manual mode, extra attributes, page props and the
  queue stub. The site id and script host fall back to `PRIVATUS_SITE_ID` and
  `PRIVATUS_SCRIPT_HOST`.
- `Config::tag()` prints the script tag, with per-page changes, a Content
  Security Policy nonce and page props. On in release builds only unless
  `enabled` is set.
- `event_attrs`, `event_data`, `click_attr`, `click_data` and `valid_click`
  for custom events and click maps.
- `page_props_json` and the `privatusPageProps` script, and the `props!` macro.
- `inject_tag` for adding the tag to a page you already have as a string.
- Feature `axum`: `PrivatusLayer`, a tower layer that adds the tag to HTML pages.
- Feature `actix`: `PrivatusMiddleware` for actix-web.
- Features `askama` and `maud`: the helpers print as HTML in those template
  engines.
- When the tag is off (not enabled, a debug build by default, or no site id) and
  the queue stub is on, `Tag::to_html` prints only the queue stub (with the nonce),
  so `privatus.track` calls never fail. No tracker and no page props script print
  then. The axum and actix-web middleware still add nothing when the tag is off.
- Add the conformance example app in `example/` (axum and askama).
- README: a privacy policy paragraph you can paste.
