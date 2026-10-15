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

Create the first admin (only possible while no admin exists; after that, only a signed-in
admin can create admins — send their token as `Authorization: Bearer …`):

```bash
curl -X POST http://localhost:8080/api/auth/signup \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","name":"Admin","password":"Admin@1501"}'
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
  bookeo/        typed Bookeo API client (products cache, slots, customers, bookings, payments)
  qnb/           QNB payment gateway client (Pay by Link, transaction inquiry, link cancel)
  email.rs       SMTP2GO mailer
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

## Security model

- **Two kinds of token, never interchangeable** (`kind` claim): admin tokens only work on admin
  routes, player tokens only on `/api/player/*` and the chat socket (403 otherwise).
- **Every request re-checks the account**: deleted accounts and tokens issued before a password
  reset are rejected (401). An admin resetting a player's password or deleting them also closes
  that player's open chat sockets.
- **Players only ever see their own data**: bookings are filtered by the token's player id; chat
  history and read receipts are scoped to conversations that include the caller; other players
  are visible only as name/playername/online status (never phone or email).
- **Logins** are throttled (5 failures per client, 20 per account, per 15 min → 429) and take the
  same time whether or not the account exists.
- **Admin cookie** is httpOnly + SameSite=Lax, and writes from other sites' pages are refused by
  an Origin check (CSRF). API responses are `Cache-Control: no-store`.
- **Admin signup** is only open while no admin exists (first-admin bootstrap).

`scripts/authz-check.mjs` verifies all of the above against a running dev server (every route ×
no token / wrong token kind / forged token / another player). Run it after changing routes:

```bash
API=http://localhost:8080/api ADMIN_PASSWORD=... node scripts/authz-check.mjs
# add WITH_BOOKINGS=1 only when the server uses a mock/test Bookeo (it creates bookings)
```

## API

Two kinds of token, never interchangeable: **admin** (web panel; httpOnly cookie or Bearer) and
**player** (mobile app; Bearer only). Errors are always `{"error": "..."}`.

### Admin

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/api/health` | — | |
| POST | `/api/auth/signup` | admin (or none for the very first admin) | `{username, name, password}` |
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

| POST | `/api/stats/imports` | admin | multipart field `file`: a game report PDF → 202, processed in the background |
| GET | `/api/stats/imports/{id}` | admin | poll until `status` is `completed` / `failed` (with players matched, unmatched names, warnings) |
| GET | `/api/stats/imports` | admin | last 10 uploads |
| GET | `/api/settings/payments` | admin | `{mode: pay_now\|pay_later, pay_later_minutes, pay_now_minutes, online_payment_enabled}` |
| PUT | `/api/settings/payments` | admin | `{mode, pay_later_minutes}` (15 min – 30 days) |
| GET | `/api/stats/leaderboard` | admin | general report: every player's all-time totals, ranked by points |
| GET | `/api/offers?search=&page=&per_page=` | admin | paginated, newest first; search matches title or description |
| POST | `/api/offers` | admin | `{title, description, is_active?, starts_on?, ends_on?}` — dates `YYYY-MM-DD`, inclusive; `is_active` defaults to `true` |
| GET | `/api/offers/{id}` | admin | |
| PUT | `/api/offers/{id}` | admin | same body |
| DELETE | `/api/offers/{id}` | admin | |

### Player (mobile app)

| Method | Path | Notes |
|---|---|---|
| POST | `/api/player/auth/login` | `{playername, password}` → `{token, player}`; send `Authorization: Bearer <token>` afterwards |
| POST | `/api/player/auth/signup` | `{full_name, playername, email, phone, password}` (email and phone required) → `{token, player}` (201), signed in. 409 if the playername is taken; at most 5 sign-ups per network per 15 min (429). Admins can still create players in the panel |
| GET | `/api/player/auth/playername-available?playername=` | `{playername, available, reason}` — for the sign-up form (format rules + taken) |
| GET | `/api/player/auth/me` | current player |
| GET | `/api/player/games` | bookable games from Bookeo (cached 10 min): price, duration, group size, `requires_approval` |
| GET | `/api/player/games/{product_id}/slots?from=YYYY-MM-DD&to=YYYY-MM-DD` | open slots with `event_id` and seats left. No dates = rest of today; `from` only = that day; max 31 days. Dates are in the business timezone |
| POST | `/api/player/bookings` | `{product_id, event_id, num_people}` → booking (201). Bookeo rejections (slot full, …) → 422 |
| GET | `/api/player/bookings?when=upcoming\|past&page=&per_page=` | the player's own bookings |
| GET | `/api/player/bookings/{id}` | one booking, status refreshed from Bookeo |
| POST | `/api/player/bookings/{id}/payment?lang=ar` | QNB payment page for the booking → `{status, pay_url, amount, currency, due_at, last_failure, …}`. Reuses the open link; a new one after a failed attempt. 409 if nothing to pay / canceled / time to pay passed |
| GET | `/api/player/bookings/{id}/payment` | payment state, confirmed with QNB — poll while the payment page is open |
| DELETE | `/api/player/bookings/{id}/payment` | the customer closed the payment page unpaid: cancels a *pay now* booking at once (a last-second payment still counts); *pay later* bookings are unaffected |

| GET | `/api/player/stats` | my all-time totals and leaderboard `position` (`null` until I've played) |
| GET | `/api/player/stats/leaderboard` | general leaderboard any player can see: top 100 by all-time points, plus `me` (my row, even if lower) and `total_ranked`. Others appear by playername only (no full name/email/phone) |
| GET | `/api/player/stats/games?page=&per_page=` | my game records, newest first — every field from my page |
| GET | `/api/player/stats/games/{game_id}` | my record in one game (404 if I didn't play it) |

| GET | `/api/player/offers` | promotions live today: active and within their dates (business timezone), newest first. Plain array of `{id, title, description, starts_on, ends_on, …}` |

### Payments (QNB, mobile app)

Bookings made in the app are paid by card on QNB's hosted payment page (spec 5.23 "Pay by Link"),
opened in a WebView. Card details never reach the app or this server. Booking responses carry
`payment_status` (`not_required` | `unpaid` | `paid`) and `payment_due_at`.

1. Create the booking (`POST /api/player/bookings`). If `payment_status` is `unpaid`:
2. `POST /api/player/bookings/{id}/payment` → open `pay_url` in a WebView.
3. While it's open, poll `GET …/payment` every 3–5 s. Close the WebView when `status` is `paid`
   (show success). If `last_failure` is set, show it and offer "Try again" (repeat step 2).
   Don't read the WebView's URL or page to decide the outcome — QNB has no return URL for
   pay links, and only the server's answer counts.
4. If the customer closes the payment page without paying, call `DELETE …/payment`. In *pay
   now* mode that cancels the booking right away; in *pay later* mode it stays `unpaid` until
   `payment_due_at` and "Pay" on the booking repeats step 2.

When QNB accepts a payment the server marks the booking `paid`, records the payment in Bookeo
(`creditCard`, comment `QNB transaction <ipgTxnId>`) and emails a receipt. A background job
(every minute) catches payments finished with the app closed, retries Bookeo if it was down, and
cancels bookings not paid by `payment_due_at` (links closed first; a last-second payment wins).

The seat is always booked before payment, so a payment can never arrive for seats that are gone
(no refunds): before an unpaid booking is canceled, its QNB link is closed and checked one last
time.

Admins choose in **Settings**: *Pay now* (default; canceled as soon as the customer leaves the
payment page unpaid, or after 10 min if the app never reports back) or *Pay later*
(a set time, 15 min – 30 days; the customer is emailed a pay link). The deadline is never later
than the game's start. Staff-made bookings have nothing to pay online. Pending-approval bookings
are paid right away; if staff deny one, refund it from the QNB merchant console.

Emails (SMTP2GO): payment due (pay later, with link), payment received, canceled for non-payment.
Customers without an email address get none.

### Game statistics (PDF import)

The arena exports a report per game (`Total_statistic_HH:MM.pdf`). Page 1 is a text table
(name, ID, points, efficiency, frags, deactivated, shots, hits, accuracy); pages 2+ are scanned
images, one per player (team, team score, scenario, nominations, the 12 personal stats incl.
Efficiency KDR, victims and insulters).

- **Reading:** page 1 is parsed as text (exact). Player pages are read by OCR (`ocrs`, pure Rust;
  models embedded in the binary by `build.rs`) at the template's fixed field positions, ~4 s/page.
  Each page is linked to its page-1 row by the in-game ID (name double-checked).
- **Trust:** fields on both pages (points, shots, hits, frags, deactivated, accuracy) are taken
  from page 1; any difference is saved as a warning on the import. Efficiency (page 1) and
  Efficiency KDR (player page) are stored separately.
- **Matching:** names are lowercased and matched to `playername`. Unknown names are kept
  (unlinked) and reported; creating a player with that playername links them automatically.
- **Safety:** one transaction per game (all or nothing), append-only (nothing is overwritten or
  deleted), duplicates refused (same file hash or same game fingerprint), uploads ≤ 25 MB, one
  import at a time. The uploaded file lives in a private temp folder and is deleted afterwards —
  success or failure.

Regression test on a real report (not in CI, needs the file):
`STATS_PDF=/path/Total_statistic_17:43.pdf cargo test --release real_report -- --ignored`

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
| POST | `/api/player/push-tokens` | `{token, platform: ios\|android}` — Expo push token, after sign-in (204) |
| DELETE | `/api/player/push-tokens` | `{token}` — on sign-out (204) |

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
phones lose signal. The app keeps the socket open only while it's in the foreground, so a
message to a player who isn't connected is sent as a **push notification** (Expo Push Service:
title = sender's playername, body = message preview, `data: {type: "chat_message", player_id}`,
Android channel `messages`). Tokens Expo reports as unregistered are deleted; a password reset
removes all of the player's tokens. Presence is tracked in memory — fine for one server process; several
instances would need a shared bus (e.g. Postgres LISTEN/NOTIFY).

Booking `status` is `pending` (awaiting staff approval — all current games have it enabled),
`confirmed` or `canceled`. Each player is mirrored as one Bookeo customer, created on their first booking.

## Production notes

- `server/.env`: `COOKIE_SECURE=true`, `CORS_ORIGINS=https://<domain>`, `BIND_ADDR=127.0.0.1:8080` behind nginx,
  plus `BOOKEO_API_KEY`, `BOOKEO_SECRET_KEY`, `BOOKEO_TIMEZONE` (see `server/.env.example`).
- Payments: `QNB_BASE_URL` (production URL from QNB), `QNB_USER_ID`, `QNB_MERCHANT_ID`,
  `QNB_MERCHANT_PWD`, `QNB_MERCHANT_KEY` (API password/key: "Merchant API Details" in the QNB
  merchant console). Without them online payment is off. Emails: `SMTP2GO_API_KEY` and
  `MAIL_FROM` (a sender on a domain verified in SMTP2GO); without them emails are only logged.
  The Bookeo API key needs permission to read and write payments.
- Push notifications work without configuration. If "enhanced push security" is enabled in the
  Expo project, set `EXPO_ACCESS_TOKEN`. (`EXPO_PUSH_URL` overrides the endpoint, for tests.)
- `client/.env.production`: `VITE_API_URL=https://api.<domain>`.
- nginx must pass WebSocket upgrades for `/api/player/chats/ws`
  (`proxy_http_version 1.1; proxy_set_header Upgrade $http_upgrade; proxy_set_header Connection "upgrade";`
  and `proxy_read_timeout` above 70 s).
