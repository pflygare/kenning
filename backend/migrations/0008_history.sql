-- Revision history. Every revision is kept; these columns say what happened
-- to each one, so the history can mark published versions, drafts that were
-- thrown away, and versions brought back from earlier ones.

ALTER TABLE page_revisions
    -- When this revision went live; it stays set after a later publish replaces it.
    ADD COLUMN published_at  timestamptz,
    ADD COLUMN published_by  uuid REFERENCES users (id),
    -- Set on a draft's revisions when someone discards the draft.
    ADD COLUMN discarded_at  timestamptz,
    -- The earlier revision this one was restored from.
    ADD COLUMN restored_from uuid REFERENCES page_revisions (id);

ALTER TABLE pages
    ADD COLUMN archived_by uuid REFERENCES users (id);

-- Fill in what the audit log already recorded.
UPDATE page_revisions r SET published_at = e.created_at, published_by = e.actor_id
FROM (SELECT DISTINCT ON (revision_id) revision_id, created_at, actor_id
      FROM audit_events WHERE action = 'page.published' AND revision_id IS NOT NULL
      ORDER BY revision_id, created_at) e
WHERE e.revision_id = r.id;

UPDATE page_revisions r SET published_at = p.published_at, published_by = p.published_by
FROM pages p
WHERE p.published_revision_id = r.id AND r.published_at IS NULL;

UPDATE page_revisions r SET discarded_at = e.created_at
FROM audit_events e
WHERE e.action = 'page.draft_discarded' AND e.revision_id = r.id;

UPDATE pages p SET archived_by = e.actor_id
FROM (SELECT DISTINCT ON (object_id) object_id, actor_id FROM audit_events
      WHERE action = 'page.archived' ORDER BY object_id, created_at DESC) e
WHERE e.object_id = p.id AND p.archived_at IS NOT NULL;
