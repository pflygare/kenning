<img src="frontend/public/logo.svg" width="64" alt="Kenning logo">

# Kenning

*A kenning is an Old Norse figure of speech, like "whale-road" for the sea.*

A team knowledge base in the spirit of [Slab](https://slab.com): a calm place to write things down, organize them, and find them again.

## Vision

- **Pages**: rich documents written in a fast, distraction-free markdown editor, with revision history. An outline docked on the right lists the headings, while reading and while editing, follows along as you scroll and links straight to a section.
- **Topics**: hierarchical collections that organize pages. A page can live in more than one topic.
- **Categories**: fixed choices such as an information class (Open, Internal, Restricted), required on every page or on pages in chosen topics.
- **Search**: full-text search across pages from a field in the top bar (Ctrl/⌘ K focuses it) that suggests pages and topics as you type, plus a search page with highlighted snippets and topic, tag and category filters. Gmail-style syntax narrows a search: `Engineering: roll back` searches inside a topic, and `topic:`, `tag:` and category names (`class:internal`) work as filters.
- **Collaboration** (later): comments, mentions, and real-time co-editing.

## Stack

| Part | Tech | Directory |
| --- | --- | --- |
| Backend | Rust, [axum](https://github.com/tokio-rs/axum), tokio, [sqlx](https://github.com/launchbadge/sqlx) | `backend/` |
| Database | PostgreSQL 17 | `backend/migrations/` |
| Frontend | TypeScript, React, [React Router](https://reactrouter.com), [Vite](https://vite.dev) | `frontend/` |

## Getting started

Prerequisites: Rust (stable), Node.js 22+, and Docker (for Postgres) or a local PostgreSQL 16+.

Start Postgres and point the backend at it:

```sh
docker compose up -d
cp .env.example backend/.env
```

Start the backend. It applies migrations on startup and listens on `http://127.0.0.1:3000`:

```sh
cd backend
cargo run
curl http://127.0.0.1:3000/api/health
```

Start the frontend dev server in another terminal (proxies `/api` to the backend):

```sh
cd frontend
npm install
npm run dev
```

Then open http://localhost:5173 and create an account. Set `PUBLIC_URL=http://localhost:5173` in `backend/.env` so emailed links point at the dev server.

Without `SMTP_URL`, emails (confirmation, password reset, invitations) are printed in the backend's log. For local testing, set `DEV_TOOLS=true` to get a Testing page at `/dev` that lists every email with clickable links and can confirm your email in one click. Never turn it on where real people sign up. Google sign-in appears once `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` are set (see `.env.example`).

For realistic test content, import a few Wikipedia articles (long text with section headings) as published pages in a Wikipedia topic. It asks for your password and skips articles that are already there:

```sh
node scripts/import-wikipedia.mjs --email you@example.com --org your-org-slug
```

Name articles as extra arguments, or pass `--lang sv` for Swedish Wikipedia.

In production the backend serves the built frontend too: run `npm run build` and set `STATIC_DIR=../frontend/dist`.

## How the backend is organized

- **Migrations** live in `backend/migrations/` and run at startup (`sqlx::migrate!`).
- **Tenancy:** organization-scoped work uses `db::begin_org`, which runs the transaction as the `kenning_app` role with the organization set, so Postgres row-level security hides other organizations' rows even if a query forgets to filter.
- **Accounts:** people sign up with email and password or with Google; a Google sign-in with the same verified address joins the existing account. Sessions are random tokens in an HttpOnly cookie (`kenning_session`), stored hashed. Requests that change data must be JSON, which, with the SameSite cookie, blocks cross-site forgery.
- **Organizations:** anyone with a confirmed email can create one and becomes its owner. Owners and admins invite people by email, whether or not they have an account yet; only owners can make owners, and the last owner cannot leave or be demoted.
- **Pages:** each page has a current revision (what editors work on) and a published revision (what readers see); when they differ, the page has an unpublished draft. Saves by the same person within 10 minutes update one revision instead of piling up new ones, and a save based on an outdated revision is refused so two editors cannot silently overwrite each other.
- **Topics and tags:** topics form a tree (`parent_id`, with `position` ordering siblings); a page can sit in several topics and carry up to 20 tags. Tags are unique per organization by slug, so names differing only in case or punctuation are one tag. Neither is versioned with page content: changes apply at once and go to the audit log.
- **Categories:** a category has ordered values, and a page has at most one value per category. Owners and admins mark a category required everywhere or in topics (which covers their sub-topics); `kenning_required_categories(page)` resolves that, and publishing a page missing a required value is refused with `missing_categories`. A value in use can't be deleted.
- **Search:** each revision has a generated `search` tsvector (title weighted above body, markdown reduced to words by `kenning_plain_text`) with a GIN index, and `pg_trgm` indexes titles and topic names for typo-tolerant matches. The `simple` configuration doesn't stem, so every language works the same; each typed word matches as a prefix. A page is searched as readers see it: its published revision, or its draft if never published.
- **Errors:** handlers return `AppResult<T>`; every error becomes `{"error": {"code", "message"}}` with a matching status.
- **Audit log:** `audit::record` appends to `audit_events` inside the same transaction as the change. Events are hash-chained per organization and the table rejects UPDATE and DELETE.
- **Jobs:** `jobs::enqueue` queues work in Postgres; `jobs::Worker` runs it with retries and backoff. Any number of servers can share the queue.
- **API types:** structs marked `#[ts(export)]` are written to `frontend/src/api/types/` when `cargo test` runs. Commit the generated files; CI fails if they are stale.

## Checks

Backend tests create a throwaway database per test, so `DATABASE_URL` must point at a user that can create databases (the Compose user can).

```sh
cd backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd frontend && npm run lint && npm run build
```
