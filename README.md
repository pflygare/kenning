# Kenning

*A kenning is an Old Norse figure of speech, like "whale-road" for the sea.*

A team knowledge base in the spirit of [Slab](https://slab.com): a calm place to write things down, organize them, and find them again.

## Vision

- **Pages**: rich documents written in a fast, distraction-free markdown editor, with revision history.
- **Topics**: hierarchical collections that organize pages. A page can live in more than one topic.
- **Search**: instant full-text search across every page and topic.
- **Collaboration** (later): comments, mentions, and real-time co-editing.

## Stack

| Part | Tech | Directory |
| --- | --- | --- |
| Backend | Rust, [axum](https://github.com/tokio-rs/axum), tokio | `backend/` |
| Frontend | TypeScript, React, [Vite](https://vite.dev) | `frontend/` |

## Getting started

Prerequisites: Rust (stable) and Node.js 22+.

Start the backend (listens on `http://127.0.0.1:3000`, override with `PORT`):

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

Then open http://localhost:5173.

## Checks

```sh
cd backend && cargo fmt --check && cargo clippy -- -D warnings && cargo test
cd frontend && npm run lint && npm run build
```
