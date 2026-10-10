-- Full-text search over page titles and bodies, plus fuzzy title matching.

CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Markdown reduced to its words: links and images keep their text, HTML tags
-- and syntax characters become spaces.
CREATE FUNCTION kenning_plain_text(md text) RETURNS text
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT regexp_replace(
               regexp_replace(
                   regexp_replace(md, '!?\[([^\]]*)\]\([^)]*\)', '\1', 'g'),
                   '<[^>]+>', ' ', 'g'),
               '[#*_`>~|\\]+', ' ', 'g')
$$;

-- The 'simple' configuration doesn't stem, so it works the same for any
-- language; searches match word prefixes instead.
ALTER TABLE page_revisions ADD COLUMN search tsvector GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', title), 'A')
    || setweight(to_tsvector('simple', kenning_plain_text(body_md)), 'B')
) STORED;

CREATE INDEX page_revisions_search ON page_revisions USING gin (search);
CREATE INDEX page_revisions_title_trgm ON page_revisions USING gin (title gin_trgm_ops);
CREATE INDEX topics_name_trgm ON topics USING gin (name gin_trgm_ops);
