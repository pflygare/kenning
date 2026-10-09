-- Categories: named sets of fixed values, such as "Information class" with
-- open, internal and restricted. A page holds at most one value per category.
-- A category can be required on every page or on pages in chosen topics
-- (including their sub-topics); a page missing a required value can be
-- edited but not published.

CREATE TABLE categories (
    id                  uuid        PRIMARY KEY,
    org_id              uuid        NOT NULL REFERENCES organizations (id),
    name                text        NOT NULL CHECK (length(name) BETWEEN 1 AND 50),
    description         text        NOT NULL DEFAULT '' CHECK (length(description) <= 500),
    required_everywhere boolean     NOT NULL DEFAULT false,
    created_by          uuid        NOT NULL REFERENCES users (id),
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX categories_name ON categories (org_id, lower(name));

CREATE TABLE category_values (
    id          uuid        PRIMARY KEY,
    org_id      uuid        NOT NULL REFERENCES organizations (id),
    category_id uuid        NOT NULL REFERENCES categories (id) ON DELETE CASCADE,
    name        text        NOT NULL CHECK (length(name) BETWEEN 1 AND 50),
    color       text        NOT NULL DEFAULT 'gray'
                CHECK (color IN ('gray', 'blue', 'green', 'amber', 'red', 'purple', 'teal', 'pink')),
    position    integer     NOT NULL DEFAULT 0,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX category_values_name ON category_values (category_id, lower(name));

-- Topics whose pages (and sub-topics' pages) must have a value in the category.
CREATE TABLE category_topics (
    org_id      uuid NOT NULL REFERENCES organizations (id),
    category_id uuid NOT NULL REFERENCES categories (id) ON DELETE CASCADE,
    topic_id    uuid NOT NULL REFERENCES topics (id),
    PRIMARY KEY (category_id, topic_id)
);

CREATE TABLE page_categories (
    org_id      uuid        NOT NULL REFERENCES organizations (id),
    page_id     uuid        NOT NULL REFERENCES pages (id),
    category_id uuid        NOT NULL REFERENCES categories (id) ON DELETE CASCADE,
    -- A value in use can't be deleted; rename it instead.
    value_id    uuid        NOT NULL REFERENCES category_values (id),
    set_by      uuid        NOT NULL REFERENCES users (id),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (page_id, category_id)
);

CREATE INDEX page_categories_value ON page_categories (value_id);

-- The categories a page must have a value in: those required everywhere, and
-- those required in any of its topics or their ancestors.
CREATE FUNCTION kenning_required_categories(p_page uuid) RETURNS TABLE (category_id uuid)
LANGUAGE sql STABLE AS $$
    WITH RECURSIVE up AS (
        SELECT t.id, t.parent_id FROM page_topics pt JOIN topics t ON t.id = pt.topic_id
        WHERE pt.page_id = p_page AND t.archived_at IS NULL
        UNION
        SELECT t.id, t.parent_id FROM topics t JOIN up ON t.id = up.parent_id
    )
    SELECT c.id FROM categories c WHERE c.required_everywhere
    UNION
    SELECT ct.category_id FROM category_topics ct JOIN up ON up.id = ct.topic_id
$$;

ALTER TABLE categories ENABLE ROW LEVEL SECURITY;
CREATE POLICY categories_tenant ON categories TO kenning_app
    USING (org_id = kenning_current_org());

ALTER TABLE category_values ENABLE ROW LEVEL SECURITY;
CREATE POLICY category_values_tenant ON category_values TO kenning_app
    USING (org_id = kenning_current_org());

ALTER TABLE category_topics ENABLE ROW LEVEL SECURITY;
CREATE POLICY category_topics_tenant ON category_topics TO kenning_app
    USING (org_id = kenning_current_org());

ALTER TABLE page_categories ENABLE ROW LEVEL SECURITY;
CREATE POLICY page_categories_tenant ON page_categories TO kenning_app
    USING (org_id = kenning_current_org());
