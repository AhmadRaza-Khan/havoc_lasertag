# Havoc Laser Tag

Monorepo for the Havoc Laser Tag platform.

| Path | What | Deployed to |
|---|---|---|
| `server/` | Rust (Axum + SQLx + PostgreSQL) API | `api.<domain>` |
| `client/` | React + Vite web app | `<domain>` |
| `.github/workflows/` | CI (and later, deploy) | — |

A React Native mobile app will consume the same API (Bearer tokens) later.

## Local development

```bash
# API — needs PostgreSQL; copy and fill in the env file first
cd server
cp .env.example .env
cargo run            # http://localhost:8080, migrations run on startup

# Web
cd client
npm install
npm run dev          # http://localhost:5173
```

Create a user (there is no signup page):

```bash
curl -X POST http://localhost:8080/api/auth/signup \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","name":"Admin","password":"change-me-123"}'
```

## Project structure

### Server — `server/src/`

```
main.rs          thin entrypoint → lib::run()
lib.rs           startup: config, db, migrations, serve
app.rs           builds the router + global middleware (CORS, tracing)
config.rs        env config
db.rs            pool + migrations
error.rs         AppError → JSON { "error": "..." } responses
state.rs         AppState shared by handlers
shared/          cross-module helpers: validation, password hashing, pagination
modules/
  mod.rs         mounts every module's routes under /api (public vs admin-only groups)
  <feature>/
    mod.rs       routes() + public exports
    dto.rs       request/response types + validation
    handlers.rs  thin HTTP layer (extract → call service → respond)
    service.rs   business logic
    repository.rs / model.rs   SQL + DB types
```

**Adding a feature:** create `modules/<feature>/` (see `modules/players/` as the reference),
expose `pub fn routes() -> Router<AppState>`, and nest it in the `admin` group in `modules/mod.rs`
— every route in that group requires an admin token. Handlers that need the caller take an
`AuthUser` argument. Add schema changes as new files in `server/migrations/` (never edit an applied one).

### Client — `client/src/`

```
main.tsx         entrypoint
app/             App shell + providers, route table (router.tsx), sidebar entries (navigation.ts)
components/
  layout/        AdminLayout, Sidebar, PageHeader
  ui/            reusable primitives: Modal, ConfirmDialog, Pagination, SearchInput
hooks/           shared hooks
lib/             api-client, react-query client, formatters
styles/          global CSS + design tokens
features/
  <feature>/
    api.ts       calls to the API for this feature
    hooks.ts     TanStack Query hooks (queries + mutations)
    types.ts
    components/  pages/
    index.ts     the feature's public exports — import via '@/features/<feature>'
```

Imports use the `@/` alias for `src/`. Server data is fetched with TanStack Query.

**Adding a feature (e.g. "Bookings"):** create `features/bookings/` following `features/players/`,
add its route inside the `AdminLayout` group in `app/router.tsx`, and add a sidebar entry in
`app/navigation.ts`.

## API

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/api/health` | — | |
| POST | `/api/auth/signup` | — | `{username, name, password}` |
| POST | `/api/auth/login` | — | `{username, password}` → sets httpOnly cookie, returns `{token, user}` |
| POST | `/api/auth/logout` | — | clears cookie |
| GET | `/api/auth/me` | cookie or `Bearer` | current user |
| GET | `/api/players?search=&page=&per_page=` | admin | paginated; search matches playername or full name |
| POST | `/api/players` | admin | `{full_name, playername, phone?, email?, password}` |
| GET | `/api/players/{id}` | admin | |
| PUT | `/api/players/{id}` | admin | same body; blank/missing `password` keeps the current one |
| DELETE | `/api/players/{id}` | admin | |

## Production notes

- `server/.env`: `COOKIE_SECURE=true`, `CORS_ORIGINS=https://<domain>`, `BIND_ADDR=127.0.0.1:8080` behind nginx.
- `client/.env.production`: `VITE_API_URL=https://api.<domain>`.
