-- Accounts and organizations. Emails and slugs are stored lowercased by the app.

-- A person, independent of any organization.
CREATE TABLE users (
    id                uuid        PRIMARY KEY,
    email             text        NOT NULL UNIQUE CHECK (email = lower(email)),
    name              text        NOT NULL,
    avatar_url        text,
    email_verified_at timestamptz,
    created_at        timestamptz NOT NULL DEFAULT now(),
    disabled_at       timestamptz
);

-- One row per way to sign in. Google and a password can both point at one user.
CREATE TABLE identities (
    id            uuid        PRIMARY KEY,
    user_id       uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider      text        NOT NULL CHECK (provider IN ('password', 'google')),
    subject       text        NOT NULL,
    password_hash text,
    created_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (provider, subject),
    CHECK ((provider = 'password') = (password_hash IS NOT NULL))
);
CREATE UNIQUE INDEX identities_one_password ON identities (user_id) WHERE provider = 'password';

-- Signed-in browsers. The cookie holds a random token; only its hash is stored.
CREATE TABLE sessions (
    token_hash   bytea       PRIMARY KEY,
    user_id      uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz NOT NULL
);
CREATE INDEX sessions_user ON sessions (user_id);

-- Single-use emailed links: email verification and password reset.
CREATE TABLE email_tokens (
    token_hash bytea       PRIMARY KEY,
    kind       text        NOT NULL CHECK (kind IN ('verify_email', 'reset_password')),
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    email      text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    used_at    timestamptz
);

-- In-flight Google sign-ins, between the redirect out and the callback.
CREATE TABLE oauth_flows (
    state         text        PRIMARY KEY,
    nonce         text        NOT NULL,
    pkce_verifier text        NOT NULL,
    next_path     text        NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE organizations (
    id         uuid        PRIMARY KEY,
    name       text        NOT NULL,
    slug       text        NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    created_by uuid        NOT NULL REFERENCES users (id),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE memberships (
    org_id     uuid        NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role       text        NOT NULL CHECK (role IN ('owner', 'admin', 'member')),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (org_id, user_id)
);
CREATE INDEX memberships_user ON memberships (user_id);

-- Invitations by email, for people with or without an account.
CREATE TABLE invites (
    id          uuid        PRIMARY KEY,
    org_id      uuid        NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    token_hash  bytea       NOT NULL UNIQUE,
    email       text        NOT NULL CHECK (email = lower(email)),
    role        text        NOT NULL CHECK (role IN ('owner', 'admin', 'member')),
    invited_by  uuid        NOT NULL REFERENCES users (id),
    created_at  timestamptz NOT NULL DEFAULT now(),
    expires_at  timestamptz NOT NULL,
    accepted_at timestamptz,
    accepted_by uuid        REFERENCES users (id),
    revoked_at  timestamptz
);
-- At most one open invite per address per organization.
CREATE UNIQUE INDEX invites_open ON invites (org_id, email)
    WHERE accepted_at IS NULL AND revoked_at IS NULL;

-- Organization-scoped tables: inside db::begin_org (role kenning_app) only the
-- current organization's rows are visible.
ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
CREATE POLICY organizations_tenant ON organizations TO kenning_app
    USING (id = kenning_current_org());

ALTER TABLE memberships ENABLE ROW LEVEL SECURITY;
CREATE POLICY memberships_tenant ON memberships TO kenning_app
    USING (org_id = kenning_current_org());

ALTER TABLE invites ENABLE ROW LEVEL SECURITY;
CREATE POLICY invites_tenant ON invites TO kenning_app
    USING (org_id = kenning_current_org());
