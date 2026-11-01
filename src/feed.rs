//! Fetching and parsing of RSS/Atom feeds into a neutral model.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use feed_rs::{model, parser};

/// One article of a feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Article {
    /// The headline (never empty).
    pub title: String,
    /// The address of the article (opened in the browser).
    pub link: Option<String>,
    /// The publication date, if the feed states one.
    pub date: Option<DateTime<Utc>>,
    /// A plain-text teaser (HTML removed), if the feed offers one.
    pub teaser: Option<String>,
}

/// A feed and its articles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feed {
    /// The feed's own title (never empty).
    pub title: String,
    /// The articles, in the order the feed lists them.
    pub articles: Vec<Article>,
}

/// The largest feed body that is read, in bytes.
///
/// A feed is fetched from a server the user does not control, so the download is bounded: a
/// hostile or broken feed that answers with gigabytes must not exhaust the program's memory.
pub const MAX_BYTES: usize = 10 * 1024 * 1024;

/// Whether an address from a feed may be handed to the system's browser.
///
/// Feed content is untrusted, so the opener only ever sees `http`/`https` addresses. Anything
/// else is refused — another scheme (`file:`, `smb:`, …) would hand the URI to whatever handler
/// is registered for it, and a value like `-x` would reach `xdg-open` as an option instead of a
/// URL.
pub fn is_openable(url: &str) -> bool {
    let url = url.trim().to_ascii_lowercase();
    url.starts_with("http://") || url.starts_with("https://")
}

/// Download the raw bytes of a feed, refusing a body above [`MAX_BYTES`].
pub async fn fetch_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let mut response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("could not reach {url}"))?
        .error_for_status()
        .with_context(|| format!("{url} answered with an error"))?;

    // Refuse a body that announces itself as too large before reading it.
    if let Some(length) = response.content_length()
        && length > MAX_BYTES as u64
    {
        return Err(too_large(url, length as usize));
    }

    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .with_context(|| format!("could not read the body of {url}"))?
    {
        extend_within_limit(&mut body, &chunk).with_context(|| format!("while reading {url}"))?;
    }
    Ok(body)
}

/// Append `chunk`, failing once the total would exceed [`MAX_BYTES`].
fn extend_within_limit(buffer: &mut Vec<u8>, chunk: &[u8]) -> Result<()> {
    if buffer.len() + chunk.len() > MAX_BYTES {
        return Err(too_large("the feed", buffer.len() + chunk.len()));
    }
    buffer.extend_from_slice(chunk);
    Ok(())
}

fn too_large(what: &str, bytes: usize) -> anyhow::Error {
    anyhow::anyhow!(
        "{what} is larger than the {} MiB limit ({bytes} bytes)",
        MAX_BYTES / (1024 * 1024)
    )
}

/// Download and parse a feed.
pub async fn fetch(client: &reqwest::Client, url: &str) -> Result<Feed> {
    let bytes = fetch_bytes(client, url).await?;
    parse_feed(&bytes)
}

/// Parse feed bytes (RSS, Atom or JSON Feed) into the neutral model.
pub fn parse_feed(bytes: &[u8]) -> Result<Feed> {
    let parsed = parser::parse(bytes).context("the feed could not be parsed")?;
    let title = parsed
        .title
        .as_ref()
        .map(|text| clean_text(&text.content))
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| "(untitled feed)".to_string());

    let articles = parsed.entries.iter().map(article_from_entry).collect();

    Ok(Feed { title, articles })
}

fn article_from_entry(entry: &model::Entry) -> Article {
    let title = entry
        .title
        .as_ref()
        .map(|text| clean_text(&text.content))
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| "(untitled article)".to_string());

    let teaser = entry
        .summary
        .as_ref()
        .map(|text| clean_text(&text.content))
        .filter(|teaser| !teaser.is_empty())
        .or_else(|| {
            entry
                .content
                .as_ref()
                .and_then(|content| content.body.as_ref())
                .map(|body| clean_text(body))
                .filter(|teaser| !teaser.is_empty())
        });

    Article {
        title,
        link: pick_link(&entry.links),
        date: entry.published.or(entry.updated),
        teaser,
    }
}

/// Prefer the `alternate` link (the human-readable page); fall back to the first.
fn pick_link(links: &[model::Link]) -> Option<String> {
    links
        .iter()
        .find(|link| link.rel.as_deref().is_none_or(|rel| rel == "alternate"))
        .or_else(|| links.first())
        .map(|link| link.href.clone())
}

/// Turn a fragment that may contain HTML into plain text.
///
/// Tags are dropped, common entities decoded, and runs of whitespace collapsed.
pub fn clean_text(input: &str) -> String {
    let mut without_tags = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if in_tag => {}
            _ => without_tags.push(ch),
        }
    }
    collapse_whitespace(&decode_entities(&without_tags))
}

fn decode_entities(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEISE: &[u8] = include_bytes!("../fixtures/heise_atom.xml");
    const BBC: &[u8] = include_bytes!("../fixtures/bbc_rss.xml");
    const TAGESSCHAU: &[u8] = include_bytes!("../fixtures/tagesschau_rss.xml");

    #[test]
    fn a_real_atom_feed_parses_into_articles() {
        let feed = parse_feed(HEISE).expect("heise atom must parse");
        assert_eq!(feed.title, "heise online News");
        assert!(!feed.articles.is_empty());
        // Every article has a headline, thanks to the fallback.
        assert!(feed.articles.iter().all(|a| !a.title.trim().is_empty()));
        // A real article carries a link, a date and a teaser.
        let article = feed
            .articles
            .iter()
            .find(|a| a.teaser.is_some())
            .expect("at least one heise article has a teaser");
        assert!(
            article
                .link
                .as_deref()
                .is_some_and(|l| l.starts_with("http"))
        );
        assert!(article.date.is_some());
        assert!(!article.teaser.as_ref().unwrap().contains('<'));
    }

    #[test]
    fn a_real_rss2_feed_parses_into_articles() {
        let feed = parse_feed(BBC).expect("bbc rss must parse");
        assert_eq!(feed.title, "BBC News");
        assert!(!feed.articles.is_empty());
        assert!(feed.articles.iter().all(|a| !a.title.trim().is_empty()));
        assert!(feed.articles.iter().any(|a| a.link.is_some()));
    }

    #[test]
    fn the_tagesschau_feed_also_parses() {
        let feed = parse_feed(TAGESSCHAU).expect("tagesschau rss must parse");
        assert!(!feed.title.trim().is_empty());
        assert!(!feed.articles.is_empty());
    }

    #[test]
    fn no_teaser_keeps_html() {
        let feed = parse_feed(BBC).unwrap();
        for article in &feed.articles {
            if let Some(teaser) = &article.teaser {
                assert!(!teaser.contains('<') && !teaser.contains('>'));
            }
        }
    }

    #[test]
    fn clean_text_removes_tags_and_decodes_entities() {
        assert_eq!(clean_text("<p>Hello <b>world</b></p>"), "Hello world");
        assert_eq!(clean_text("a &amp; b"), "a & b");
        assert_eq!(clean_text("line\n  one   two"), "line one two");
        assert_eq!(clean_text("&lt;tag&gt;"), "<tag>");
        assert_eq!(clean_text("x&nbsp;y"), "x y");
    }

    #[test]
    fn broken_input_is_an_error_not_a_silent_empty_feed() {
        assert!(parse_feed(b"this is not a feed").is_err());
    }

    #[test]
    fn only_http_addresses_may_be_opened() {
        // What a feed legitimately links to.
        assert!(is_openable("https://example.org/article"));
        assert!(is_openable("http://example.org/article"));
        assert!(is_openable("  HTTPS://EXAMPLE.ORG/x  ")); // trimmed, case-insensitive

        // Everything else stays with the program: another scheme would hand the URI to a
        // different handler, and a leading dash would reach the opener as an option.
        assert!(!is_openable("file:///etc/passwd"));
        assert!(!is_openable("smb://attacker.example/share"));
        assert!(!is_openable("javascript:alert(1)"));
        assert!(!is_openable("mailto:someone@example.org"));
        assert!(!is_openable("-x"));
        assert!(!is_openable("--version"));
        assert!(!is_openable(""));
        assert!(!is_openable("example.org/no-scheme"));
    }

    #[test]
    fn a_body_above_the_limit_is_refused() {
        let mut buffer = Vec::new();
        extend_within_limit(&mut buffer, &vec![b'a'; MAX_BYTES]).unwrap();
        assert_eq!(buffer.len(), MAX_BYTES);

        // One byte more must fail, and the buffer must stay as it was.
        let error = extend_within_limit(&mut buffer, b"x").unwrap_err();
        assert!(error.to_string().contains("10 MiB"), "got: {error}");
        assert_eq!(buffer.len(), MAX_BYTES);

        // A fresh buffer accepts a small chunk.
        let mut small = Vec::new();
        extend_within_limit(&mut small, b"hello").unwrap();
        assert_eq!(small, b"hello");
    }
}
