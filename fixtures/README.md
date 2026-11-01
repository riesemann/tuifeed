# Test fixtures

Recorded feed payloads. The parser tests in `src/feed.rs` and the render test in
`tests/last_article_visible.rs` run against these bytes instead of hand-written examples, so the
code meets what a server actually sends: Atom and RSS 2.0, CDATA sections, HTML in the summaries,
`&amp;`-style entities and all.

| File | Source | Format | Entries |
|---|---|---|---|
| `heise_atom.xml` | heise online News (`https://www.heise.de/rss/heise-atom.xml`) | Atom 1.0 | 153 |
| `bbc_rss.xml` | BBC News, news front page | RSS 2.0 | 27 |
| `tagesschau_rss.xml` | tagesschau.de, `Die Nachrichten der ARD` | RSS 2.0 | 40 |

Each file is one complete response, fetched on 2026-10-04 and committed unchanged — nothing in
them was edited by hand. The headlines, teasers and links are the respective publishers' content;
they are kept here only as a fixed input for the tests, not as a redistribution of the feeds.

To replace one: record a fresh response from the same address, and keep the assertions in
`src/feed.rs` (`a_real_atom_feed_parses_into_articles`, `a_real_rss2_feed_parses_into_articles`,
`the_tagesschau_feed_also_parses`) in step with the new content — they pin titles, counts and the
presence of links, dates and teasers.
