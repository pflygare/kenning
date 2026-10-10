//! Search: full-text over page titles and bodies, and a quick title lookup.
//!
//! A page is searched as readers see it: its published revision, or its draft
//! if it was never published. Every word typed matches as a prefix, so results
//! appear while someone is still typing; titles also match with typos.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection, types::Json};
use ts_rs::TS;
use uuid::Uuid;

use crate::{categories::PageValue, tags::TagRef, topics::TopicRef};

/// Words past this many are ignored.
const MAX_WORDS: usize = 10;
const MAX_QUERY: usize = 200;

/// Marks the start and end of a matched word in `title` and `snippet`.
pub const MARK_START: char = '\u{2}';
pub const MARK_END: char = '\u{3}';

#[derive(Debug, Default, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: String,
    /// Only pages in this topic or its sub-topics.
    pub topic: Option<Uuid>,
    /// A tag's slug.
    pub tag: Option<String>,
    /// A category value's id.
    pub value: Option<Uuid>,
}

/// A page that matched, best first.
#[derive(Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct SearchHit {
    pub short_id: String,
    pub slug: String,
    /// The title with matches between `\u0002` and `\u0003`.
    pub title: String,
    /// Words around the matches in the body, marked the same way.
    pub snippet: String,
    pub published: bool,
    pub updated_at: DateTime<Utc>,
    pub updated_by_name: String,
    #[ts(as = "Vec<TopicRef>")]
    pub topics: Json<Vec<TopicRef>>,
    #[ts(as = "Vec<TagRef>")]
    pub tags: Json<Vec<TagRef>>,
    #[ts(as = "Vec<PageValue>")]
    pub values: Json<Vec<PageValue>>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SearchResults {
    /// Topics whose names match.
    pub topics: Vec<TopicRef>,
    pub pages: Vec<SearchHit>,
}

/// A page title for the quick-open box.
#[derive(Debug, Serialize, FromRow, TS)]
#[ts(export)]
pub struct QuickPage {
    pub short_id: String,
    pub slug: String,
    pub title: String,
    pub published: bool,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct QuickResults {
    pub topics: Vec<TopicRef>,
    pub pages: Vec<QuickPage>,
}

/// The words of a query as a prefix-matching tsquery, such as `deplo:* & run:*`.
/// Only letters and digits survive, so the result is always valid tsquery syntax.
pub fn prefix_query(q: &str) -> Option<String> {
    let words: Vec<String> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .take(MAX_WORDS)
        .map(|w| format!("{}:*", w.to_lowercase()))
        .collect();
    (!words.is_empty()).then(|| words.join(" & "))
}

fn trimmed(q: &str) -> String {
    q.trim().chars().take(MAX_QUERY).collect()
}

const HIGHLIGHT: &str = "'StartSel=' || chr(2) || ', StopSel=' || chr(3)";

pub async fn search(conn: &mut PgConnection, query: &SearchQuery) -> sqlx::Result<SearchResults> {
    let raw = trimmed(&query.q);
    let Some(tsquery) = prefix_query(&raw) else {
        return Ok(SearchResults {
            topics: vec![],
            pages: vec![],
        });
    };
    let filtered = query.topic.is_some() || query.tag.is_some() || query.value.is_some();
    let topics = if filtered {
        vec![]
    } else {
        sqlx::query_as(
            "SELECT id, short_id, slug, name FROM topics
             WHERE archived_at IS NULL
               AND (to_tsvector('simple', name) @@ to_tsquery('simple', $1) OR name % $2)
             ORDER BY similarity(name, $2) DESC, lower(name)
             LIMIT 5",
        )
        .bind(&tsquery)
        .bind(&raw)
        .fetch_all(&mut *conn)
        .await?
    };
    let pages = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "WITH q AS (SELECT to_tsquery('simple', $1) AS q),
         hits AS (
             SELECT p.id, p.short_id, p.slug, p.published_revision_id, p.updated_at,
                    r.title, r.body_md, r.author_id,
                    ts_rank_cd(r.search, q.q) + similarity(r.title, $2) AS rank
             FROM pages p
             JOIN page_revisions r ON r.id = coalesce(p.published_revision_id, p.current_revision_id)
             CROSS JOIN q
             WHERE p.archived_at IS NULL
               AND (r.search @@ q.q
                    OR r.title % $2
                    OR EXISTS (SELECT 1 FROM page_tags pt JOIN tags t ON t.id = pt.tag_id
                               WHERE pt.page_id = p.id AND to_tsvector('simple', t.name) @@ q.q))
               AND ($3::uuid IS NULL OR EXISTS (
                     WITH RECURSIVE under AS (
                         SELECT id FROM topics WHERE id = $3
                         UNION
                         SELECT t.id FROM topics t JOIN under ON t.parent_id = under.id
                     )
                     SELECT 1 FROM page_topics pt JOIN under ON under.id = pt.topic_id
                     WHERE pt.page_id = p.id))
               AND ($4::text IS NULL OR EXISTS (
                     SELECT 1 FROM page_tags pt JOIN tags t ON t.id = pt.tag_id
                     WHERE pt.page_id = p.id AND t.slug = $4))
               AND ($5::uuid IS NULL OR EXISTS (
                     SELECT 1 FROM page_categories pc WHERE pc.page_id = p.id AND pc.value_id = $5))
             ORDER BY rank DESC, p.updated_at DESC
             LIMIT 50
         )
         SELECT h.short_id, h.slug,
                ts_headline('simple', h.title, q.q, {HIGHLIGHT} || ', HighlightAll=true') AS title,
                ts_headline('simple', kenning_plain_text(h.body_md), q.q,
                            {HIGHLIGHT} || ', MaxWords=30, MinWords=15, MaxFragments=2, FragmentDelimiter=\" … \"')
                    AS snippet,
                h.published_revision_id IS NOT NULL AS published,
                h.updated_at, u.name AS updated_by_name,
                coalesce((SELECT json_agg(json_build_object('id', t.id, 'short_id', t.short_id,
                                                            'slug', t.slug, 'name', t.name)
                                          ORDER BY lower(t.name))
                          FROM page_topics pt JOIN topics t ON t.id = pt.topic_id
                          WHERE pt.page_id = h.id AND t.archived_at IS NULL),
                         '[]') AS topics,
                coalesce((SELECT json_agg(json_build_object('id', t.id, 'name', t.name, 'slug', t.slug,
                                                            'color', t.color) ORDER BY lower(t.name))
                          FROM page_tags pt JOIN tags t ON t.id = pt.tag_id WHERE pt.page_id = h.id),
                         '[]') AS tags,
                coalesce((SELECT json_agg(json_build_object('category', c.name, 'id', v.id, 'name', v.name,
                                                            'color', v.color) ORDER BY lower(c.name))
                          FROM page_categories pc
                          JOIN categories c ON c.id = pc.category_id
                          JOIN category_values v ON v.id = pc.value_id
                          WHERE pc.page_id = h.id),
                         '[]') AS values
         FROM hits h CROSS JOIN q JOIN users u ON u.id = h.author_id
         ORDER BY h.rank DESC, h.updated_at DESC"
    )))
    .bind(&tsquery)
    .bind(&raw)
    .bind(query.topic)
    .bind(&query.tag)
    .bind(query.value)
    .fetch_all(&mut *conn)
    .await?;
    Ok(SearchResults { topics, pages })
}

/// Titles for the quick-open box: matching pages and topics, or the most
/// recently changed pages when nothing is typed yet.
pub async fn quick(conn: &mut PgConnection, q: &str) -> sqlx::Result<QuickResults> {
    let raw = trimmed(q);
    let Some(tsquery) = prefix_query(&raw) else {
        let pages = sqlx::query_as(
            "SELECT p.short_id, p.slug, coalesce(pub.title, cur.title) AS title,
                    p.published_revision_id IS NOT NULL AS published
             FROM pages p
             JOIN page_revisions cur ON cur.id = p.current_revision_id
             LEFT JOIN page_revisions pub ON pub.id = p.published_revision_id
             WHERE p.archived_at IS NULL
             ORDER BY p.updated_at DESC LIMIT 8",
        )
        .fetch_all(&mut *conn)
        .await?;
        return Ok(QuickResults {
            topics: vec![],
            pages,
        });
    };
    let topics = sqlx::query_as(
        "SELECT id, short_id, slug, name FROM topics
         WHERE archived_at IS NULL
           AND (to_tsvector('simple', name) @@ to_tsquery('simple', $1) OR name % $2)
         ORDER BY lower(name) LIKE lower($2) || '%' DESC, similarity(name, $2) DESC, lower(name)
         LIMIT 4",
    )
    .bind(&tsquery)
    .bind(&raw)
    .fetch_all(&mut *conn)
    .await?;
    let pages = sqlx::query_as(
        "SELECT short_id, slug, title, published FROM (
             SELECT p.short_id, p.slug, coalesce(pub.title, cur.title) AS title,
                    p.published_revision_id IS NOT NULL AS published, p.updated_at
             FROM pages p
             JOIN page_revisions cur ON cur.id = p.current_revision_id
             LEFT JOIN page_revisions pub ON pub.id = p.published_revision_id
             WHERE p.archived_at IS NULL
         ) p
         WHERE to_tsvector('simple', title) @@ to_tsquery('simple', $1) OR title % $2
         ORDER BY lower(title) LIKE lower($2) || '%' DESC, similarity(title, $2) DESC, updated_at DESC
         LIMIT 8",
    )
    .bind(&tsquery)
    .bind(&raw)
    .fetch_all(&mut *conn)
    .await?;
    Ok(QuickResults { topics, pages })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_become_prefix_matches() {
        assert_eq!(
            prefix_query("Deplo run").as_deref(),
            Some("deplo:* & run:*")
        );
        assert_eq!(prefix_query("  e-mail ").as_deref(), Some("e:* & mail:*"));
        assert_eq!(
            prefix_query("Åsa's ÖL").as_deref(),
            Some("åsa:* & s:* & öl:*")
        );
        assert_eq!(prefix_query("'):* | !"), None);
        assert_eq!(
            prefix_query(&"a ".repeat(50)).unwrap().matches(':').count(),
            MAX_WORDS
        );
    }
}
