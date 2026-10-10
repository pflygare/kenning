//! Search: full-text over page titles and bodies, and a quick title lookup.
//!
//! A page is searched as readers see it: its published revision, or its draft
//! if it was never published. Every word typed matches as a prefix, so results
//! appear while someone is still typing; titles also match with typos.
//!
//! A query can start with a topic's name to search inside it, as in
//! `Engineering: roll back`. It can also narrow the search with
//! `topic:Engineering`, `tag:how-to`, or
//! a category and value such as `class:internal`. Quote names with spaces:
//! `topic:"Backend services"`, `"information class":open`.

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
    /// What the query's filters were understood as, such as "Topic: Engineering".
    pub scope: Vec<String>,
    /// Filters in the query that matched nothing, such as `topic:nope`.
    pub unmatched: Vec<String>,
}

/// One `key:value` filter written in a query.
#[derive(Debug, PartialEq, Eq)]
pub struct Filter {
    pub key: String,
    pub value: String,
    /// As written, for messages.
    pub text: String,
}

/// Split a query into its `key:value` filters and the words left over.
/// Keys and values may be quoted to hold spaces. Whether a key means anything
/// is decided later; `keep` says which keys to take out of the text.
pub fn parse(q: &str, keep: impl Fn(&str) -> bool) -> (Vec<Filter>, String) {
    let mut filters = vec![];
    let mut rest = String::new();
    let chars: Vec<char> = q.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            rest.push(' ');
            i += 1;
            continue;
        }
        let start = i;
        let (key, after_key) = read_part(&chars, i, true);
        if after_key < chars.len() && chars[after_key] == ':' && !key.is_empty() && keep(&key) {
            let (value, end) = read_part(&chars, after_key + 1, false);
            if !value.trim().is_empty() {
                filters.push(Filter {
                    key: key.to_lowercase(),
                    value: value.trim().to_string(),
                    text: chars[start..end].iter().collect(),
                });
                i = end;
                continue;
            }
        }
        // Not a filter: keep this word as text.
        while i < chars.len() && !chars[i].is_whitespace() {
            rest.push(chars[i]);
            i += 1;
        }
    }
    (
        filters,
        rest.split_whitespace().collect::<Vec<_>>().join(" "),
    )
}

/// A quoted string, or a run of characters up to whitespace (or `:` for a key).
fn read_part(chars: &[char], start: usize, is_key: bool) -> (String, usize) {
    if chars.get(start) == Some(&'"')
        && let Some(len) = chars[start + 1..].iter().position(|&c| c == '"')
    {
        let end = start + 1 + len;
        return (chars[start + 1..end].iter().collect(), end + 1);
    }
    let mut end = start;
    while end < chars.len() && !chars[end].is_whitespace() && !(is_key && chars[end] == ':') {
        end += 1;
    }
    (chars[start..end].iter().collect(), end)
}

/// A category name as typed in a filter key: case, spaces and punctuation don't matter.
fn key_form(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
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

/// Filters resolved to what they name.
#[derive(Default)]
struct Scope {
    topics: Vec<Uuid>,
    tags: Vec<String>,
    values: Vec<Uuid>,
    labels: Vec<String>,
    unmatched: Vec<String>,
}

/// The topic a name refers to: an exact name or slug first, else the shortest
/// name starting with it.
async fn find_topic(conn: &mut PgConnection, name: &str) -> sqlx::Result<Option<(Uuid, String)>> {
    sqlx::query_as(
        "SELECT id, name FROM topics
         WHERE archived_at IS NULL
           AND (lower(name) = lower($1) OR slug = $2 OR lower(name) LIKE lower($1) || '%')
         ORDER BY lower(name) = lower($1) OR slug = $2 DESC, length(name), lower(name)
         LIMIT 1",
    )
    .bind(name)
    .bind(crate::orgs::slugify(name))
    .fetch_optional(conn)
    .await
}

/// A query written as `Topic name: words` searches inside that topic. Returns
/// the topic and the rest of the query when the text before the first colon
/// names one.
async fn topic_first(
    conn: &mut PgConnection,
    q: &str,
) -> sqlx::Result<Option<((Uuid, String), String)>> {
    let Some((head, tail)) = q.split_once(':') else {
        return Ok(None);
    };
    let head = head.trim();
    let key = head.to_lowercase();
    if head.is_empty()
        || head.contains('"')
        || key == "topic"
        || key == "tag"
        || tail.starts_with("//")
    {
        return Ok(None);
    }
    Ok(find_topic(conn, head)
        .await?
        .map(|topic| (topic, tail.trim().to_string())))
}

/// Category names in their filter-key form, so `class:` or `information-class:` can name one.
async fn category_keys(conn: &mut PgConnection) -> sqlx::Result<Vec<(Uuid, String, String)>> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as("SELECT id, name FROM categories")
        .fetch_all(conn)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name)| (id, key_form(&name), name))
        .collect())
}

async fn resolve(
    conn: &mut PgConnection,
    filters: &[Filter],
    categories: &[(Uuid, String, String)],
) -> sqlx::Result<Scope> {
    let mut scope = Scope::default();
    for filter in filters {
        let slug = crate::orgs::slugify(&filter.value);
        let found: Option<(Uuid, String)> = match filter.key.as_str() {
            "topic" => find_topic(&mut *conn, &filter.value).await?,
            "tag" => {
                sqlx::query_as("SELECT id, name FROM tags WHERE slug = $1")
                    .bind(&slug)
                    .fetch_optional(&mut *conn)
                    .await?
            }
            key => {
                let category = categories
                    .iter()
                    .find(|(_, form, _)| form == &key_form(key));
                match category {
                    Some((category_id, _, _)) => {
                        sqlx::query_as(
                            "SELECT id, name FROM category_values
                         WHERE category_id = $1
                           AND (lower(name) = lower($2) OR lower(name) LIKE lower($2) || '%')
                         ORDER BY lower(name) = lower($2) DESC, position
                         LIMIT 1",
                        )
                        .bind(category_id)
                        .bind(&filter.value)
                        .fetch_optional(&mut *conn)
                        .await?
                    }
                    None => None,
                }
            }
        };
        let Some((id, name)) = found else {
            scope.unmatched.push(filter.text.clone());
            continue;
        };
        match filter.key.as_str() {
            "topic" => {
                scope.topics.push(id);
                scope.labels.push(format!("Topic: {name}"));
            }
            "tag" => {
                scope.tags.push(slug);
                scope.labels.push(format!("Tag: {name}"));
            }
            key => {
                let category = categories
                    .iter()
                    .find(|(_, form, _)| form == &key_form(key))
                    .map(|(_, _, name)| name.as_str())
                    .unwrap_or(key);
                scope.values.push(id);
                scope.labels.push(format!("{category}: {name}"));
            }
        }
    }
    Ok(scope)
}

pub async fn search(conn: &mut PgConnection, query: &SearchQuery) -> sqlx::Result<SearchResults> {
    let categories = category_keys(&mut *conn).await?;
    let mut raw = trimmed(&query.q);
    let first = topic_first(&mut *conn, &raw).await?;
    if let Some((_, rest)) = &first {
        raw = rest.clone();
    }
    let (filters, text) = parse(&raw, |key| {
        let key = key.to_lowercase();
        key == "topic"
            || key == "tag"
            || categories
                .iter()
                .any(|(_, form, _)| *form == key_form(&key))
    });
    let mut scope = resolve(&mut *conn, &filters, &categories).await?;
    if let Some(((id, name), _)) = first {
        scope.topics.insert(0, id);
        scope.labels.insert(0, format!("Topic: {name}"));
    }
    scope.topics.extend(query.topic);
    scope.tags.extend(query.tag.clone());
    scope.values.extend(query.value);
    let tsquery = prefix_query(&text);
    let empty = SearchResults {
        topics: vec![],
        pages: vec![],
        scope: scope.labels.clone(),
        unmatched: scope.unmatched.clone(),
    };
    // Nothing to look for, or a filter that names nothing: no pages, not all of them.
    if (tsquery.is_none() && scope.labels.is_empty()) || !scope.unmatched.is_empty() {
        return Ok(empty);
    }
    let filtered = !scope.topics.is_empty() || !scope.tags.is_empty() || !scope.values.is_empty();
    let topics = match (&tsquery, filtered) {
        (Some(tsquery), false) => {
            sqlx::query_as(
                "SELECT id, short_id, slug, name FROM topics
                 WHERE archived_at IS NULL
                   AND (to_tsvector('simple', name) @@ to_tsquery('simple', $1) OR name % $2)
                 ORDER BY similarity(name, $2) DESC, lower(name)
                 LIMIT 5",
            )
            .bind(tsquery)
            .bind(&text)
            .fetch_all(&mut *conn)
            .await?
        }
        _ => vec![],
    };
    let pages = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "WITH q AS (SELECT to_tsquery('simple', $1) AS q),
         hits AS (
             SELECT p.id, p.short_id, p.slug, p.published_revision_id, p.updated_at,
                    r.title, r.body_md, r.author_id,
                    coalesce(ts_rank_cd(r.search, q.q), 0) + similarity(r.title, $2) AS rank
             FROM pages p
             JOIN page_revisions r ON r.id = coalesce(p.published_revision_id, p.current_revision_id)
             CROSS JOIN q
             WHERE p.archived_at IS NULL
               AND ($1::text IS NULL
                    OR r.search @@ q.q
                    OR r.title % $2
                    OR EXISTS (SELECT 1 FROM page_tags pt JOIN tags t ON t.id = pt.tag_id
                               WHERE pt.page_id = p.id AND to_tsvector('simple', t.name) @@ q.q))
               -- Every topic named: the page is in it or one of its sub-topics.
               AND NOT EXISTS (
                     SELECT 1 FROM unnest($3::uuid[]) AS wanted(id)
                     WHERE NOT EXISTS (
                         WITH RECURSIVE under AS (
                             SELECT wanted.id AS id
                             UNION
                             SELECT t.id FROM topics t JOIN under ON t.parent_id = under.id
                         )
                         SELECT 1 FROM page_topics pt JOIN under ON under.id = pt.topic_id
                         WHERE pt.page_id = p.id))
               AND NOT EXISTS (
                     SELECT 1 FROM unnest($4::text[]) AS wanted(slug)
                     WHERE NOT EXISTS (
                         SELECT 1 FROM page_tags pt JOIN tags t ON t.id = pt.tag_id
                         WHERE pt.page_id = p.id AND t.slug = wanted.slug))
               AND NOT EXISTS (
                     SELECT 1 FROM unnest($5::uuid[]) AS wanted(id)
                     WHERE NOT EXISTS (
                         SELECT 1 FROM page_categories pc
                         WHERE pc.page_id = p.id AND pc.value_id = wanted.id))
             ORDER BY rank DESC, p.updated_at DESC
             LIMIT 50
         )
         SELECT h.short_id, h.slug,
                coalesce(ts_headline('simple', h.title, q.q, {HIGHLIGHT} || ', HighlightAll=true'),
                         h.title) AS title,
                coalesce(ts_headline('simple', kenning_plain_text(h.body_md), q.q,
                                     {HIGHLIGHT} || ', MaxWords=30, MinWords=15, MaxFragments=2, FragmentDelimiter=\" … \"'),
                         left(kenning_plain_text(h.body_md), 200)) AS snippet,
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
    .bind(&text)
    .bind(&scope.topics)
    .bind(&scope.tags)
    .bind(&scope.values)
    .fetch_all(&mut *conn)
    .await?;
    Ok(SearchResults {
        topics,
        pages,
        ..empty
    })
}

/// Titles for the quick-open box: matching pages and topics, or the most
/// recently changed pages when nothing is typed yet.
pub async fn quick(conn: &mut PgConnection, q: &str) -> sqlx::Result<QuickResults> {
    let raw = trimmed(q);
    // A query that narrows by topic, tag or category is for full search, not titles.
    let categories = category_keys(&mut *conn).await?;
    let (filters, _) = parse(&raw, |key| {
        let key = key.to_lowercase();
        key == "topic"
            || key == "tag"
            || categories
                .iter()
                .any(|(_, form, _)| *form == key_form(&key))
    });
    if !filters.is_empty() || topic_first(&mut *conn, &raw).await?.is_some() {
        return Ok(QuickResults {
            topics: vec![],
            pages: vec![],
        });
    }
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
    fn filters_come_out_of_the_text() {
        let known = |key: &str| ["topic", "tag", "class"].contains(&key.to_lowercase().as_str());
        let (filters, text) = parse("Topic:Engineering deploy tag:how-to", known);
        assert_eq!(text, "deploy");
        assert_eq!(
            filters
                .iter()
                .map(|f| (f.key.as_str(), f.value.as_str()))
                .collect::<Vec<_>>(),
            [("topic", "Engineering"), ("tag", "how-to")]
        );
        assert_eq!(filters[1].text, "tag:how-to");

        let (filters, text) = parse("topic:\"Backend services\" roll back", known);
        assert_eq!(filters[0].value, "Backend services");
        assert_eq!(text, "roll back");

        // Unknown keys, empty values and stray colons stay text.
        let (filters, text) = parse("see https://x.io at 10:30 topic: class:open", known);
        assert_eq!(text, "see https://x.io at 10:30 topic:");
        assert_eq!(filters.len(), 1);
        assert_eq!(key_form("Information class"), key_form("information-class"));
    }

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
