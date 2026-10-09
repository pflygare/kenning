-- Topics organize pages into a tree; tags are flat labels. A page can sit in
-- several topics and carry several tags. Neither is versioned with the page's
-- content: adding a page to a topic or tagging it takes effect at once.

CREATE TABLE topics (
    id          uuid        PRIMARY KEY,
    org_id      uuid        NOT NULL REFERENCES organizations (id),
    -- URLs are /{org}/t/{slug}-{short_id}, like pages.
    short_id    text        NOT NULL CHECK (short_id ~ '^[a-z0-9]{10}$'),
    slug        text        NOT NULL,
    parent_id   uuid        REFERENCES topics (id),
    name        text        NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    description text        NOT NULL DEFAULT '' CHECK (length(description) <= 1000),
    -- Orders siblings; ties fall back to the name.
    position    integer     NOT NULL DEFAULT 0,
    created_by  uuid        NOT NULL REFERENCES users (id),
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    archived_at timestamptz,
    UNIQUE (org_id, short_id),
    CHECK (parent_id IS DISTINCT FROM id)
);

CREATE INDEX topics_parent ON topics (org_id, parent_id, position) WHERE archived_at IS NULL;

CREATE TABLE page_topics (
    org_id     uuid        NOT NULL REFERENCES organizations (id),
    page_id    uuid        NOT NULL REFERENCES pages (id),
    topic_id   uuid        NOT NULL REFERENCES topics (id),
    added_by   uuid        NOT NULL REFERENCES users (id),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (page_id, topic_id)
);

CREATE INDEX page_topics_topic ON page_topics (topic_id);

CREATE TABLE tags (
    id         uuid        PRIMARY KEY,
    org_id     uuid        NOT NULL REFERENCES organizations (id),
    name       text        NOT NULL CHECK (length(name) BETWEEN 1 AND 50),
    -- "Onboarding" and "onboarding" are one tag.
    slug       text        NOT NULL,
    color      text        NOT NULL DEFAULT 'gray'
               CHECK (color IN ('gray', 'blue', 'green', 'amber', 'red', 'purple', 'teal', 'pink')),
    created_by uuid        NOT NULL REFERENCES users (id),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (org_id, slug)
);

CREATE TABLE page_tags (
    org_id     uuid        NOT NULL REFERENCES organizations (id),
    page_id    uuid        NOT NULL REFERENCES pages (id),
    -- Deleting a tag removes it from every page.
    tag_id     uuid        NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    added_by   uuid        NOT NULL REFERENCES users (id),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (page_id, tag_id)
);

CREATE INDEX page_tags_tag ON page_tags (tag_id);

ALTER TABLE topics ENABLE ROW LEVEL SECURITY;
CREATE POLICY topics_tenant ON topics TO kenning_app
    USING (org_id = kenning_current_org());

ALTER TABLE page_topics ENABLE ROW LEVEL SECURITY;
CREATE POLICY page_topics_tenant ON page_topics TO kenning_app
    USING (org_id = kenning_current_org());

ALTER TABLE tags ENABLE ROW LEVEL SECURITY;
CREATE POLICY tags_tenant ON tags TO kenning_app
    USING (org_id = kenning_current_org());

ALTER TABLE page_tags ENABLE ROW LEVEL SECURITY;
CREATE POLICY page_tags_tenant ON page_tags TO kenning_app
    USING (org_id = kenning_current_org());
