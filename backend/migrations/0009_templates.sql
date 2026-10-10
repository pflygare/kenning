-- Templates: starting points for new pages. Starting a page from one copies
-- its title, text, topics and tags into the page, filling placeholders such as
-- {{date}}; later changes to the template never touch pages made from it.

CREATE TABLE templates (
    id          uuid        PRIMARY KEY,
    org_id      uuid        NOT NULL REFERENCES organizations (id),
    name        text        NOT NULL CHECK (length(name) BETWEEN 1 AND 100),
    description text        NOT NULL DEFAULT '' CHECK (length(description) <= 500),
    -- The new page's title; empty lets the writer pick one.
    title       text        NOT NULL DEFAULT '' CHECK (length(title) <= 200),
    body_md     text        NOT NULL DEFAULT '' CHECK (length(body_md) <= 1000000),
    created_by  uuid        NOT NULL REFERENCES users (id),
    updated_by  uuid        NOT NULL REFERENCES users (id),
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    archived_at timestamptz
);

CREATE INDEX templates_org ON templates (org_id, lower(name)) WHERE archived_at IS NULL;

CREATE TABLE template_topics (
    org_id      uuid NOT NULL REFERENCES organizations (id),
    template_id uuid NOT NULL REFERENCES templates (id) ON DELETE CASCADE,
    topic_id    uuid NOT NULL REFERENCES topics (id),
    PRIMARY KEY (template_id, topic_id)
);

CREATE TABLE template_tags (
    org_id      uuid NOT NULL REFERENCES organizations (id),
    template_id uuid NOT NULL REFERENCES templates (id) ON DELETE CASCADE,
    -- Deleting a tag removes it from templates too.
    tag_id      uuid NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (template_id, tag_id)
);

-- The template a page was started from, if any.
ALTER TABLE pages ADD COLUMN template_id uuid REFERENCES templates (id);

ALTER TABLE templates ENABLE ROW LEVEL SECURITY;
CREATE POLICY templates_tenant ON templates TO kenning_app
    USING (org_id = kenning_current_org());

ALTER TABLE template_topics ENABLE ROW LEVEL SECURITY;
CREATE POLICY template_topics_tenant ON template_topics TO kenning_app
    USING (org_id = kenning_current_org());

ALTER TABLE template_tags ENABLE ROW LEVEL SECURITY;
CREATE POLICY template_tags_tenant ON template_tags TO kenning_app
    USING (org_id = kenning_current_org());
