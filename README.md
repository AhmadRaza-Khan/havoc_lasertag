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
shared/          cross-module helpers: validation, password hashing, JWT, pagination
integrations/
  bookeo/        typed Bookeo API client (products cache, slots, customers, bookings)
modules/
  mod.rs         mounts every module's routes under /api (public / admin-only / player-only groups)
  <feature>/
    mod.rs       routes() + public exports
    dto.rs       request/response types + validation
    handlers.rs  thin HTTP layer (extract → call service → respond)
    service.rs   business logic
    repository.rs / model.rs   SQL + DB types
```

**Adding a feature:** create `modules/<feature>/` (see `modules/players/` as the reference),
expose `pub fn routes() -> Router<AppState>`, and nest it in `modules/mod.rs`: in the `admin`
group (admin token required; take an `AuthUser` argument for the caller) or the `player` group
(player token required; take an `AuthPlayer` argument). Add schema changes as new files in `server/migrations/` (never edit an applied one).

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

Two kinds of token, never interchangeable: **admin** (web panel; httpOnly cookie or Bearer) and
**player** (mobile app; Bearer only). Errors are always `{"error": "..."}`.

### Admin

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
| GET | `/api/games` | admin | same as the player games endpoint |
| GET | `/api/games/{product_id}/slots?from=&to=` | admin | same as the player slots endpoint |
| GET | `/api/bookings?search=&player_id=&when=upcoming\|past&status=&page=&per_page=` | admin | all bookings with their player; search matches player name, playername or booking number |
| POST | `/api/bookings` | admin | `{player_id, product_id, event_id, num_people}` — books for any player |
| GET | `/api/bookings/{id}` | admin | status refreshed from Bookeo |
| PUT | `/api/bookings/{id}` | admin | `{event_id, num_people}` — move to another slot of the same game / change group size |
| DELETE | `/api/bookings/{id}` | admin | cancels in Bookeo (seats freed); the booking stays listed as `canceled` |

Admin booking changes are sent to Bookeo as staff actions (`mode=backend`) without notifying staff;
player bookings notify staff. Customers get Bookeo's emails when they have an email address.

### Player (mobile app)

| Method | Path | Notes |
|---|---|---|
| POST | `/api/player/auth/login` | `{playername, password}` → `{token, player}`; send `Authorization: Bearer <token>` afterwards |
| GET | `/api/player/auth/me` | current player |
| GET | `/api/player/games` | bookable games from Bookeo (cached 10 min): price, duration, group size, `requires_approval` |
| GET | `/api/player/games/{product_id}/slots?from=YYYY-MM-DD&to=YYYY-MM-DD` | open slots with `event_id` and seats left. No dates = rest of today; `from` only = that day; max 31 days. Dates are in the business timezone |
| POST | `/api/player/bookings` | `{product_id, event_id, num_people}` → booking (201). Bookeo rejections (slot full, …) → 422 |
| GET | `/api/player/bookings?when=upcoming\|past&page=&per_page=` | the player's own bookings |
| GET | `/api/player/bookings/{id}` | one booking, status refreshed from Bookeo |

### Chat (mobile app, player token)

Private one-to-one chat between any two players. Messages persist in Postgres; delivery,
typing, read receipts and online status are real-time over one WebSocket per app session.

| Method | Path | Notes |
|---|---|---|
| GET | `/api/player/chats/players?search=&page=&per_page=` | every other player: `id, full_name, playername, online, last_seen_at` (no contact details) |
| GET | `/api/player/chats/players/{id}` | one player with presence |
| GET | `/api/player/chats/conversations` | my conversations, most recent first, with `last_message` and `unread_count` |
| GET | `/api/player/chats/conversations/{player_id}/messages?before=&limit=` | history with that player, newest first; `before` = oldest message id you have; `has_more` |
| POST | `/api/player/chats/conversations/{player_id}/messages` | `{body}` (1–2000 chars) — fallback when the socket is down; still pushed live |
| POST | `/api/player/chats/conversations/{player_id}/read` | mark their messages read (sends them a read receipt) |
| GET (WS) | `/api/player/chats/ws` | auth: `Authorization: Bearer <token>` header, or `?token=` for clients that can't set headers |

WebSocket frames are JSON with a `type`:

- **App → server:** `{"type":"send","to":<player_id>,"body":"…","client_id":"…"}`,
  `{"type":"typing","to":<player_id>,"is_typing":true}`, `{"type":"read","with":<player_id>}`, `{"type":"ping"}`
- **Server → app:** `ready {player_id, online_player_ids}` (on connect), `message {message, client_id?}`
  (to both sides; `client_id` only on the sender's copy), `typing {from, is_typing}`,
  `read {by, read_at}`, `presence {player_id, online, last_seen_at}` (to everyone), `pong`,
  `error {message, client_id?}`

App guidance: reconnect with backoff when the socket drops, then refetch conversations; treat
`typing` as stopped if not refreshed within ~6 s (re-send `true` every ~3 s while typing). The
server pings every 25 s and drops connections silent for 70 s, so presence stays accurate when
phones lose signal. Presence is tracked in memory — fine for one server process; several
instances would need a shared bus (e.g. Postgres LISTEN/NOTIFY).

Booking `status` is `pending` (awaiting staff approval — all current games have it enabled),
`confirmed` or `canceled`. Each player is mirrored as one Bookeo customer, created on their first booking.

## Production notes

- `server/.env`: `COOKIE_SECURE=true`, `CORS_ORIGINS=https://<domain>`, `BIND_ADDR=127.0.0.1:8080` behind nginx,
  plus `BOOKEO_API_KEY`, `BOOKEO_SECRET_KEY`, `BOOKEO_TIMEZONE` (see `server/.env.example`).
- `client/.env.production`: `VITE_API_URL=https://api.<domain>`.
- nginx must pass WebSocket upgrades for `/api/player/chats/ws`
  (`proxy_http_version 1.1; proxy_set_header Upgrade $http_upgrade; proxy_set_header Connection "upgrade";`
  and `proxy_read_timeout` above 70 s).
