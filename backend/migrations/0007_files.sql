-- Files uploaded into pages, for now only images. They live in the database
-- so app servers stay stateless and backups cover them; pages refer to them by
-- URL (/api/files/{id}) in their markdown. A file is never changed, only added.

CREATE TABLE files (
    id           uuid        PRIMARY KEY,
    org_id       uuid        NOT NULL REFERENCES organizations (id),
    uploaded_by  uuid        NOT NULL REFERENCES users (id),
    name         text        NOT NULL CHECK (length(name) BETWEEN 1 AND 200),
    content_type text        NOT NULL
                 CHECK (content_type IN ('image/png', 'image/jpeg', 'image/gif', 'image/webp')),
    size         integer     NOT NULL CHECK (size > 0),
    data         bytea       NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX files_org ON files (org_id, created_at);

ALTER TABLE files ENABLE ROW LEVEL SECURITY;
CREATE POLICY files_tenant ON files TO kenning_app
    USING (org_id = kenning_current_org());
