-- Pages and their revisions.
--
-- Every save writes (or updates) a page_revisions row. A page points at two of
-- them: current_revision_id is what editors work on, published_revision_id is
-- what readers see. When they differ the page has an unpublished draft.

CREATE TABLE pages (
    id                    uuid        PRIMARY KEY,
    org_id                uuid        NOT NULL REFERENCES organizations (id),
    -- Random and permanent; URLs are /{org}/p/{slug}-{short_id}, so renaming a
    -- page (which changes slug) never breaks a link.
    short_id              text        NOT NULL CHECK (short_id ~ '^[a-z0-9]{10}$'),
    slug                  text        NOT NULL,
    current_revision_id   uuid        NOT NULL,
    published_revision_id uuid,
    published_at          timestamptz,
    published_by          uuid        REFERENCES users (id),
    created_by            uuid        NOT NULL REFERENCES users (id),
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),
    archived_at           timestamptz,
    UNIQUE (org_id, short_id)
);

CREATE INDEX pages_recent ON pages (org_id, updated_at DESC) WHERE archived_at IS NULL;

CREATE TABLE page_revisions (
    id         uuid        PRIMARY KEY,
    org_id     uuid        NOT NULL REFERENCES organizations (id),
    page_id    uuid        NOT NULL REFERENCES pages (id),
    title      text        NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    body_md    text        NOT NULL CHECK (length(body_md) <= 1000000),
    author_id  uuid        NOT NULL REFERENCES users (id),
    created_at timestamptz NOT NULL DEFAULT now(),
    -- Saves by the same author in quick succession update one revision.
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX page_revisions_page ON page_revisions (page_id, created_at DESC);

ALTER TABLE pages
    ADD FOREIGN KEY (current_revision_id) REFERENCES page_revisions (id) DEFERRABLE INITIALLY DEFERRED,
    ADD FOREIGN KEY (published_revision_id) REFERENCES page_revisions (id);

ALTER TABLE pages ENABLE ROW LEVEL SECURITY;
CREATE POLICY pages_tenant ON pages TO kenning_app
    USING (org_id = kenning_current_org());

ALTER TABLE page_revisions ENABLE ROW LEVEL SECURITY;
CREATE POLICY page_revisions_tenant ON page_revisions TO kenning_app
    USING (org_id = kenning_current_org());
