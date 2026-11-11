//! Renders README.md into the HTML page served at "/" (the API's docs), so
//! the README stays the one source of truth. No runtime fetches, no CDN assets.

use pulldown_cmark::{html, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// GitHub-style anchor: lowercase, spaces to "-", punctuation dropped.
fn slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
            _ => None,
        })
        .collect()
}

fn main() {
    println!("cargo:rerun-if-changed=README.md");
    let md = std::fs::read_to_string("README.md").expect("README.md");
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;

    // Give h2/h3 headings ids, so in-page links like #countries work.
    let events: Vec<Event> = Parser::new_ext(&md, opts).collect();
    let mut out = Vec::with_capacity(events.len());
    let mut i = 0;
    while i < events.len() {
        if let Event::Start(Tag::Heading { level, .. }) = &events[i] {
            let level = *level;
            let mut text = String::new();
            let mut j = i + 1;
            while !matches!(events[j], Event::End(TagEnd::Heading(_))) {
                if let Event::Text(t) | Event::Code(t) = &events[j] {
                    text.push_str(t);
                }
                j += 1;
            }
            out.push(Event::Start(Tag::Heading {
                level,
                id: matches!(level, HeadingLevel::H2 | HeadingLevel::H3)
                    .then(|| CowStr::from(slug(&text))),
                classes: vec![],
                attrs: vec![],
            }));
            i += 1;
            continue;
        }
        out.push(events[i].clone());
        i += 1;
    }
    let mut body = String::new();
    html::push_html(&mut body, out.into_iter());

    let page = format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Simplesolat API</title>
<meta name="description" content="Free REST API for official prayer times, from the open simplesolat-data dataset.">
<style>
body{{font:16px/1.55 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;max-width:860px;margin:0 auto;padding:24px 18px 60px;color:#1b1b1b}}
h1{{font-size:2rem;margin:.2em 0}} h2{{margin-top:2em;border-bottom:1px solid #e3e3e3;padding-bottom:.2em}} h3{{margin-top:1.6em}}
a{{color:#0b6e4f}} code{{font:.9em ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;background:#f3f4f4;padding:.1em .3em;border-radius:4px}}
pre{{background:#f6f8f8;padding:12px 14px;border-radius:8px;overflow-x:auto}} pre code{{background:none;padding:0}}
table{{border-collapse:collapse;display:block;overflow-x:auto;margin:1em 0}} th,td{{border:1px solid #ddd;padding:6px 10px;text-align:left}} th{{background:#f3f6f5}}
blockquote{{margin:0;padding:0 1em;color:#555;border-left:4px solid #cfe3d8}} hr{{border:0;border-top:1px solid #e3e3e3;margin:2em 0}}
@media (prefers-color-scheme:dark){{body{{background:#121212;color:#e6e6e6}}a{{color:#6fd3a8}}code,pre{{background:#1e1e1e}}th{{background:#1f2a25}}th,td{{border-color:#333}}h2,hr{{border-color:#333}}}}
</style></head><body>
{body}
</body></html>
"#
    );
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(format!("{out_dir}/index.html"), page).unwrap();
}
