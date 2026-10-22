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
  skipcash/      SkipCash card payments client (create/read payments, request and webhook signatures)
  (modules/webhooks/  Bookeo's signed calls: bookings canceled in Bookeo)
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
| PUT | `/api/bookings/{id}` | admin | `{event_id, num_people, other_player_ids?, options?}` — move to another slot of the same game / change group size / add-ons (omitted `options` keeps the current ones; they're re-sent to Bookeo, which replaces the whole booking) |
| DELETE | `/api/bookings/{id}` | admin | cancels in Bookeo (seats freed); the booking stays listed as `canceled` |

Admin booking changes are sent to Bookeo as staff actions (`mode=backend`) without notifying staff;
player bookings notify staff. Customers get Bookeo's emails when they have an email address.

| POST | `/api/stats/imports` | admin | multipart field `file`: a game report PDF → 202, processed in the background |
| GET | `/api/stats/imports/{id}` | admin | poll until `status` is `completed` / `failed` (with players matched, unmatched names, warnings) |
| GET | `/api/stats/imports` | admin | last 10 uploads |
| GET | `/api/settings/payments` | admin | `{mode: pay_now\|pay_later, pay_later_minutes, pay_now_minutes, online_payment_enabled}` |
| PUT | `/api/settings/payments` | admin | `{mode, pay_later_minutes}` (15 min – 30 days) |
| GET | `/api/settings/skipcash` | admin | `{configured, environment, client_id, key_id, key_secret_set, webhook_key_set, updated_at}` (secrets are never returned) |
| PUT | `/api/settings/skipcash` | admin | `{environment: sandbox\|production, client_id, key_id, key_secret, webhook_key}`; empty secrets keep the saved ones. Takes effect immediately |
| DELETE | `/api/settings/skipcash` | admin | Removes the credentials: online payment off |
| POST | `/api/settings/skipcash/test` | admin | Same body as PUT, not saved. Creates a 1.00 QAR test payment (checks key ID + secret) and reads it back (checks client ID) → `{ok, checks: [{name, ok, message}]}` |
| GET | `/api/stats/leaderboard` | admin | general report: every player's all-time totals, ranked by points |
| GET | `/api/stats/players/{id}` | admin | one player's all-time totals (the admin panel's player stats page) |
| GET | `/api/stats/players/{id}/games?page=&per_page=` | admin | that player's game records, newest first, with everything from the reports |
| GET | `/api/offers?search=&page=&per_page=` | admin | paginated, newest first; search matches title or description |
| POST | `/api/offers` | admin | `{title?, description?, is_active?, starts_on?, ends_on?}` — dates `YYYY-MM-DD`, inclusive; `is_active` defaults to `true`. Text is optional: an offer can be text, an image, or both (the panel requires one of them) |
| GET | `/api/offers/{id}` | admin | |
| PUT | `/api/offers/{id}` | admin | same body; clearing the text is refused if the offer has no image |
| DELETE | `/api/offers/{id}` | admin | |
| PUT | `/api/offers/{id}/image` | admin | multipart field `file`: JPEG/PNG/WebP (checked by content), max 900 KB — the panel resizes to 1600 px first → updated offer |
| DELETE | `/api/offers/{id}/image` | admin | removes the image (refused if the offer has no text) → updated offer |
| GET | `/api/offer-images/{id}?v=…` | public | the image, cached for a year; offers carry it as `image_url` (a path) |

### Player (mobile app)

| Method | Path | Notes |
|---|---|---|
| POST | `/api/player/auth/login` | `{playername, password}` → `{token, player}`; send `Authorization: Bearer <token>` afterwards |
| POST | `/api/player/auth/signup` | `{full_name, playername, email, phone, password}` (email and phone required) → `{token, player}` (201), signed in. 409 if the playername is taken; at most 5 sign-ups per network per 15 min (429). Admins can still create players in the panel |
| GET | `/api/player/auth/playername-available?playername=` | `{playername, available, reason}` — for the sign-up form (format rules + taken) |
| GET | `/api/player/auth/me` | current player |
| DELETE | `/api/player/auth/me` | `{password}` → 204: deletes the signed-in player's account (store requirement): account, bookings in the app, chats, picture, push tokens; game stats stay, unlinked. Wrong password → 403 (not 401, so the app stays signed in); throttled like login |
| POST | `/api/player/auth/delete-account` | `{playername, password}` → 204: the same, for the public deletion page (no app needed) |
| PUT | `/api/player/me/avatar` | profile picture: multipart field `file`, JPEG/PNG/WebP (checked by content), max 900 KB (the app sends a ~1024 px JPEG) → updated player |
| DELETE | `/api/player/me/avatar` | remove the picture → updated player |
| GET | `/api/avatars/{player_id}?v=…` | the picture (no token; cached for a year — the `v` changes with each upload). Players, chat players and leaderboard entries carry it as `avatar_url` (a path: prefix the API origin) |
| GET | `/api/player/games` | bookable games from Bookeo (cached 10 min): price, duration, group size, `requires_approval`, and `options` — the game's add-ons (Bookeo options, including the business's global ones): `{id, kind: choice\|number\|on_off\|text, name, description (plain text), staff_only, required, values[{id, name, description}], min, max, default}` in display order. Bookeo adds their prices to the booking |
| GET | `/api/player/games/{product_id}/slots?from=YYYY-MM-DD&to=YYYY-MM-DD` | open slots with `event_id` and seats left. No dates = rest of today; `from` only = that day; max 31 days. Dates are in the business timezone |
| POST | `/api/player/bookings` | `{product_id, event_id, num_people, other_player_ids?, options?: [{id, value}]}` → booking (201). Add-on values: the choice value's `id`, a number, `"true"`/`"false"`, or text; staff-only add-ons are refused. Bookings return `options: [{id, name, value}]` (choice: the value's name). Bookeo rejections (slot full, …) → 422 |
| GET | `/api/player/bookings?when=upcoming\|past&page=&per_page=` | the player's own bookings |
| GET | `/api/player/bookings/{id}` | one booking, status refreshed from Bookeo |
| POST | `/api/player/bookings/{id}/payment` | SkipCash payment page for the booking → `{status, pay_url, amount, currency, due_at, last_failure, …}`. Reuses the open payment while it's payable; a new one after it expired or failed. 409 if nothing to pay / canceled / time to pay passed |
| GET | `/api/player/bookings/{id}/payment` | payment state, confirmed with SkipCash — poll while the payment page is open |
| DELETE | `/api/player/bookings/{id}/payment` | the customer closed the payment page unpaid: cancels a *pay now* booking at once (a last-second payment still counts); *pay later* bookings are unaffected |

| GET | `/api/player/stats` | my all-time totals and leaderboard `position` (`null` until I've played), plus `details` (below) |
| GET | `/api/player/stats/leaderboard` | general leaderboard any player can see: top 30 by all-time points, plus `me` (my row, even if lower) and `total_ranked`. Others appear by playername only (no full name/email/phone) |
| GET | `/api/player/stats/games?page=&per_page=` | my game records, newest first — every field from my page |
| GET | `/api/player/stats/games/{game_id}` | my record in one game (404 if I didn't play it) |
| GET | `/api/player/stats/players/{id}` | another player's totals, opened from the leaderboard (playername only; 404 if they haven't played), plus `details` |
| GET | `/api/player/stats/players/{id}/games?page=&per_page=` | that player's game records |
| GET | `/api/player/stats/players/{id}/games/{game_id}` | that player's record in one game |

`details` (also on the admin panel's `GET /api/stats/players/{id}`; not on the leaderboards) is
the personal report page over all games: `captures` and `hits_taken` (totals),
`efficiency_kdr` (average per game), `max_combo` (best), `place_in_team` / `place_in_game`
(`{best, average}`), `medals` (`[{name, count}]`, the nominations), and the 5 players tagged
most (`victims`) and who tagged them most (`insulters`). Values are `null` when never recorded.

Display names: the admin panel and the app show `points` as **Score**, `frags` as **Kills** and
`deactivated` as **KIA**, and don't show the number of games; the API keeps the original fields.

| GET | `/api/player/offers` | promotions live today: active and within their dates (business timezone), newest first. Plain array of `{id, title, description, image_url, starts_on, ends_on, …}` (title/description may be empty; offers with neither text nor image are left out). The app shows them in the top bar and on the Offers tab |

### Payments (SkipCash, mobile app)

Bookings made in the app are paid by card on SkipCash's hosted payment page (SkipCash Integration
Manual v1.4, "Card Checkout API"), opened in a WebView. Card details never reach the app or this
server. Booking responses carry `payment_status` (`not_required` | `unpaid` | `paid`) and
`payment_due_at`.

1. Create the booking (`POST /api/player/bookings`). If `payment_status` is `unpaid`:
2. `POST /api/player/bookings/{id}/payment` → open `pay_url` in a WebView.
3. While it's open, poll `GET …/payment` every 3–5 s. Close the WebView when `status` is `paid`
   (show success). If `last_failure` is set, show it and offer "Try again" (repeat step 2).
   Don't read the WebView's URL or page to decide the outcome: only the server's answer counts.
4. If the customer closes the payment page without paying, call `DELETE …/payment`. In *pay
   now* mode that cancels the booking right away; in *pay later* mode it stays `unpaid` until
   `payment_due_at` and "Pay" on the booking repeats step 2.

The server creates each payment with `POST /api/v1/payments` (signed: HMAC-SHA256 of the fields
with the key secret) with the player's name, email and phone, our reference as `transactionId`
and the booking number as `custom1`, and confirms it with `GET /api/v1/payments/{id}` (client
ID). When it's paid, the booking is marked `paid`, the payment is recorded in Bookeo
(`creditCard`, comment `SkipCash payment <id>`) and a receipt is emailed.

**Webhook**: set `https://<api domain>/api/webhooks/skipcash` as the webhook URL in the SkipCash
portal (Sandbox and Production → Credentials). Calls are verified with the saved webhook key
(bad signature → 401) and answered 200 otherwise; they settle payments finished with the app
closed and record failed attempts for the app. Leave the portal's *return URL* empty: the app
watches the payment from the server, not the page.

A background job (every minute) asks SkipCash about open payments, retries Bookeo if it was
down, and cancels bookings not paid by `payment_due_at` (a last-second payment wins). SkipCash
can't disable a payment page — it cancels unfinished ones after an hour — so before an unpaid
booking is canceled its payment is checked one last time, and payments we stopped using are
re-checked for 75 minutes: a late payment is still recorded, and if its booking was canceled
the server logs that it needs a refund from the SkipCash merchant portal. The seat is always
booked before payment, so a payment never arrives for seats that are gone.

Admins choose in **Settings**: *Pay now* (default; canceled as soon as the customer leaves the
payment page unpaid, or after 10 min if the app never reports back) or *Pay later*
(a set time, 15 min – 30 days; the customer is emailed a pay link). The deadline is never later
than the game's start. Staff-made bookings have nothing to pay online. Pending-approval bookings
are paid right away; if staff deny one, refund it from the SkipCash merchant portal.

Emails (SMTP2GO): payment due (pay later, with link), payment received, canceled for non-payment.
Customers without an email address get none.

### Bookeo webhook (bookings canceled in Bookeo)

| Method | Path | Notes |
|---|---|---|
| POST | `/api/webhooks/bookeo/bookings-deleted` | called by Bookeo (no token): a booking was canceled or deleted in Bookeo → the matching booking here is marked `canceled`. 204 when handled (also for bookings that aren't ours or are already canceled); 401 if the signature is wrong or the timestamp is more than 5 min off; 404 when `BOOKEO_WEBHOOK_URL` isn't set |

Bookeo signs each call: `X-Bookeo-Signature` = hex HMAC-SHA256, keyed with the Bookeo
**secret key** (`BOOKEO_SECRET_KEY`), of `X-Bookeo-Timestamp` + `X-Bookeo-MessageId` + the
webhook URL exactly as registered + the raw body. Nothing is sent back to Bookeo (the seats are
already free there); if the booking had an open SkipCash payment, the payments worker stops using
it within a minute. Repeat deliveries are harmless.

Set `BOOKEO_WEBHOOK_URL` to the public URL of the endpoint (production only). At startup the
server **adds** the "bookings / deleted" webhook to the Bookeo API key if it isn't there yet. It
never removes or replaces webhooks: the others on the key (bookings created/updated, payments
created → `havoc.cyberboost.io`) are left alone, and if a "bookings / deleted" webhook already
points elsewhere it is kept and an error is logged. Bookeo delivers all webhooks of an API key in
one queue, so a failing endpoint delays the others too; blocked or delayed webhooks are logged at
startup.

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

### Chat safety (store requirements)

Players can block and report (App Store guideline 1.2, Google Play UGC policy); offensive words
in messages are masked automatically (`shared/profanity.rs`).

| Method | Path | Notes |
|---|---|---|
| POST / DELETE | `/api/player/chats/players/{id}/block` | block / unblock. Blocked either way → sending is refused (403) and the blocked player disappears from the blocker's chat list and directory. Havoc Admin can't be blocked |
| GET | `/api/player/chats/blocks` | players I blocked |
| POST | `/api/player/chats/reports` | `{player_id, message_id?, reason: spam\|harassment\|inappropriate\|other, details?}` → 201; keeps a copy of the reported message |
| GET | `/api/chats/reports?status=open\|resolved` | admin: reports, newest first |
| POST | `/api/chats/reports/{id}/resolve` | admin: mark handled |

Public pages (admin web app, no login): `/privacy`, `/terms`, `/delete-account`. Store listing
answers: `STORE_SUBMISSION.md`.

### Chats as Havoc Admin (admin panel)

Staff chat with players from the panel's **Chats** page as the **Havoc Admin** player
(`playername` `admin`, fixed id `00000000-0000-0000-0000-00000000ad01`, migration 0013). It's a
normal player to the app — players see its messages, can find it and reply — but it can't sign
in and is hidden from the panel's Players list (and can't be edited or deleted). All staff share
this one inbox; the panel polls instead of using a socket.

| Method | Path | Notes |
|---|---|---|
| GET | `/api/chats/conversations` | Havoc Admin's conversations, most recent first (with unread counts) |
| GET | `/api/chats/players?search=&page=&per_page=` | every player, for starting a chat |
| GET | `/api/chats/conversations/{player_id}/messages?before=&limit=` | history, newest first |
| POST | `/api/chats/conversations/{player_id}/messages` | `{body}` → sent as Havoc Admin (live to the app, push if it's closed) |
| POST | `/api/chats/conversations/{player_id}/read` | marks the player's messages read (they get the read receipt) |

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
- `BOOKEO_WEBHOOK_URL=https://<api domain>/api/webhooks/bookeo/bookings-deleted` syncs bookings
  canceled in Bookeo (see "Bookeo webhook"). Production only: never set it on a dev server that
  uses the production Bookeo keys.
- Payments: enter the SkipCash credentials in the admin panel (Settings → SkipCash: merchant
  portal → Production → Credentials), press **Test connection**, then **Save**; without them
  online payment is off. Set the webhook URL shown there in the portal (see Payments). Emails:
  `SMTP2GO_API_KEY` and `MAIL_FROM` (a sender on a domain verified in SMTP2GO); without them
  emails are only logged. The Bookeo API key needs permission to read and write payments.
- Push notifications work without configuration. If "enhanced push security" is enabled in the
  Expo project, set `EXPO_ACCESS_TOKEN`. (`EXPO_PUSH_URL` overrides the endpoint, for tests.)
- `client/.env.production`: `VITE_API_URL=https://api.<domain>`.
- nginx must pass WebSocket upgrades for `/api/player/chats/ws`
  (`proxy_http_version 1.1; proxy_set_header Upgrade $http_upgrade; proxy_set_header Connection "upgrade";`
  and `proxy_read_timeout` above 70 s).
