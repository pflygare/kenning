-- Foundations: the application role, tenant scoping helper, audit log and job queue.

-- Requests scoped to one organization run as this role (via SET LOCAL ROLE), so
-- row-level security applies even when the connection itself is a superuser.
DO $$
BEGIN
    CREATE ROLE kenning_app NOLOGIN;
EXCEPTION
    WHEN duplicate_object THEN NULL;
    -- Concurrent test databases can race on this cluster-wide role.
    WHEN unique_violation THEN NULL;
END
$$;

-- Let the migrating (and serving) user switch into the role. Superusers can
-- already; a managed-database owner with CREATEROLE needs the grant.
DO $$
BEGIN
    EXECUTE format('GRANT kenning_app TO %I', current_user);
EXCEPTION
    WHEN OTHERS THEN NULL;
END
$$;

GRANT USAGE ON SCHEMA public TO kenning_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO kenning_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT USAGE, SELECT ON SEQUENCES TO kenning_app;

-- The organization the current transaction is scoped to, or NULL.
CREATE FUNCTION kenning_current_org() RETURNS uuid
    LANGUAGE sql STABLE
    AS $$ SELECT nullif(current_setting('app.org_id', true), '')::uuid $$;

-- Append-only, hash-chained record of everything that happens.
-- Each organization (and the system, org_id NULL) has its own chain.
CREATE TABLE audit_events (
    seq         bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    id          uuid        NOT NULL UNIQUE,
    org_id      uuid,
    actor_id    uuid,
    action      text        NOT NULL,
    object_kind text,
    object_id   uuid,
    revision_id uuid,
    details     jsonb       NOT NULL DEFAULT '{}'::jsonb,
    ip          inet,
    created_at  timestamptz NOT NULL,
    prev_hash   bytea,
    hash        bytea       NOT NULL
);

CREATE INDEX audit_events_org_seq ON audit_events (org_id, seq);
CREATE INDEX audit_events_object ON audit_events (object_kind, object_id);

CREATE FUNCTION audit_events_immutable() RETURNS trigger
    LANGUAGE plpgsql
    AS $$ BEGIN RAISE EXCEPTION 'audit_events is append-only'; END $$;

CREATE TRIGGER audit_events_no_update_or_delete
    BEFORE UPDATE OR DELETE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION audit_events_immutable();

ALTER TABLE audit_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_events FORCE ROW LEVEL SECURITY;
CREATE POLICY audit_events_tenant ON audit_events
    USING (org_id IS NOT DISTINCT FROM kenning_current_org())
    WITH CHECK (org_id IS NOT DISTINCT FROM kenning_current_org());
REVOKE UPDATE, DELETE ON audit_events FROM kenning_app;

-- Background jobs, claimed with FOR UPDATE SKIP LOCKED so any number of
-- workers can share the queue.
CREATE TABLE jobs (
    id           uuid        PRIMARY KEY,
    kind         text        NOT NULL,
    payload      jsonb       NOT NULL DEFAULT '{}'::jsonb,
    run_at       timestamptz NOT NULL DEFAULT now(),
    attempts     integer     NOT NULL DEFAULT 0,
    max_attempts integer     NOT NULL DEFAULT 5,
    locked_at    timestamptz,
    last_error   text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    done_at      timestamptz
);

CREATE INDEX jobs_ready ON jobs (run_at) WHERE done_at IS NULL;
