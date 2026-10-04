#!/usr/bin/env node
// Access-control regression check for the API: authentication on every route, and that no
// player can see another player's data. Run against a local/dev server — never production.
//
//   API=http://localhost:8080/api ADMIN_USERNAME=admin ADMIN_PASSWORD=... node scripts/authz-check.mjs
//
// WITH_BOOKINGS=1 also creates bookings to check booking isolation. That books real slots in
// whatever Bookeo account the server uses, so only enable it against a mock/test Bookeo.
//
// Creates temporary players and deletes them at the end. Needs Node 22+ (global WebSocket).

import http from 'node:http'

const API = (process.env.API ?? 'http://localhost:8080/api').replace(/\/$/, '')
const ADMIN_USERNAME = process.env.ADMIN_USERNAME ?? 'admin'
const ADMIN_PASSWORD = process.env.ADMIN_PASSWORD
const WITH_BOOKINGS = process.env.WITH_BOOKINGS === '1'
const FAKE_ID = '00000000-0000-4000-8000-000000000000'

if (!ADMIN_PASSWORD) {
  console.error('Set ADMIN_PASSWORD (and ADMIN_USERNAME if not "admin").')
  process.exit(2)
}

let passed = 0
const failures = []
function check(ok, label) {
  if (ok) passed++
  else failures.push(label)
  console.log(`${ok ? '✓' : '✗'} ${label}`)
}

// Each request gets its own fake client IP so login throttling from one check can't
// interfere with another (the API trusts X-Real-IP because only nginx can reach it).
let ipCounter = 0
const freshIp = () => `198.51.100.${(ipCounter++ % 250) + 1}`

async function call(method, path, { token, cookie, body, headers = {} } = {}) {
  const res = await fetch(API + path, {
    method,
    headers: {
      'X-Real-IP': freshIp(),
      ...(body !== undefined && { 'Content-Type': 'application/json' }),
      ...(token && { Authorization: `Bearer ${token}` }),
      ...(cookie && { Cookie: cookie }),
      ...headers,
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  const text = await res.text()
  let json = null
  try { json = text ? JSON.parse(text) : null } catch { /* not JSON */ }
  return { status: res.status, json, text, headers: res.headers }
}

/** Status of a WebSocket handshake (101 = upgraded). */
function wsHandshake(query = '', headers = {}) {
  const url = new URL(`${API}/player/chats/ws${query}`)
  return new Promise((resolve, reject) => {
    const req = http.request({
      host: url.hostname, port: url.port, path: url.pathname + url.search,
      headers: {
        Connection: 'Upgrade', Upgrade: 'websocket', 'Sec-WebSocket-Version': '13',
        'Sec-WebSocket-Key': 'dGhlIHNhbXBsZSBub25jZQ==', ...headers,
      },
    })
    req.on('upgrade', (res, socket) => { socket.destroy(); resolve(101) })
    req.on('response', (res) => { res.resume(); resolve(res.statusCode) })
    req.on('error', reject)
    req.end()
  })
}

/** Connected chat socket that records events. */
async function chatSocket(token) {
  const ws = new WebSocket(`${API.replace(/^http/, 'ws')}/player/chats/ws?token=${token}`)
  const events = []
  let closed = false
  ws.onmessage = (m) => events.push(JSON.parse(m.data))
  ws.onclose = () => { closed = true }
  await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej })
  const waitFor = async (pred, ms = 3000) => {
    const end = Date.now() + ms
    while (Date.now() < end) { if (pred()) return true; await new Promise(r => setTimeout(r, 50)) }
    return false
  }
  return { ws, events, isClosed: () => closed, waitFor }
}

/** Fails the check if any response body leaks secret or internal fields. */
const SECRET_FIELDS = ['password_hash', 'password', 'bookeo_customer_id', 'password_changed_at']
function leaks(json) {
  const s = JSON.stringify(json ?? {})
  return SECRET_FIELDS.filter((f) => s.includes(`"${f}"`))
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

const adminLogin = await call('POST', '/auth/login', { body: { username: ADMIN_USERNAME, password: ADMIN_PASSWORD } })
if (adminLogin.status !== 200) {
  console.error(`Admin login failed (${adminLogin.status}): ${adminLogin.text}`)
  process.exit(2)
}
const admin = adminLogin.json.token
const adminCookie = adminLogin.headers.get('set-cookie')?.split(';')[0]

const suffix = Date.now().toString(36)
const created = []
async function makePlayer(tag) {
  const playername = `authz_${tag}_${suffix}`
  const r = await call('POST', '/players', {
    token: admin,
    body: { full_name: `Authz ${tag}`, playername, email: `${tag}.${suffix}@example.com`, phone: '+974 5000 0000', password: 'password123' },
  })
  if (r.status !== 201) throw new Error(`creating ${playername}: ${r.status} ${r.text}`)
  created.push(r.json.id)
  const login = await call('POST', '/player/auth/login', { body: { playername, password: 'password123' } })
  return { id: r.json.id, playername, token: login.json.token }
}

const A = await makePlayer('a')
const B = await makePlayer('b')
const C = await makePlayer('c')

// Some data owned by A and B
const msg = await call('POST', `/player/chats/conversations/${B.id}/messages`, { token: A.token, body: { body: 'private A→B' } })
let bookingA = null
let bookingB = null
if (WITH_BOOKINGS) {
  const games = (await call('GET', '/player/games', { token: A.token })).json
  for (const g of games) {
    const slots = (await call('GET', `/player/games/${g.product_id}/slots?to=${new Date(Date.now() + 6 * 864e5).toISOString().slice(0, 10)}`, { token: A.token })).json
    const slot = slots?.slots?.find((s) => s.seats_available >= 2 * (g.min_people ?? 1))
    if (!slot) continue
    const body = { product_id: g.product_id, event_id: slot.event_id, num_people: g.min_people ?? 1 }
    bookingA = (await call('POST', '/player/bookings', { token: A.token, body })).json
    bookingB = (await call('POST', '/player/bookings', { token: B.token, body })).json
    break
  }
  if (!bookingA?.id || !bookingB?.id) throw new Error('could not create test bookings')
}

// ---------------------------------------------------------------------------
// 1. Every protected route requires the right kind of token
// ---------------------------------------------------------------------------

const adminRoutes = [
  ['GET', '/auth/me'],
  ['GET', '/players'], ['GET', `/players/${A.id}`], ['POST', '/players', {}],
  ['PUT', `/players/${A.id}`, {}], ['DELETE', `/players/${FAKE_ID}`],
  ['GET', '/games'], ['GET', '/games/X/slots'],
  ['GET', '/bookings'], ['GET', `/bookings/${FAKE_ID}`], ['POST', '/bookings', {}],
  ['PUT', `/bookings/${FAKE_ID}`, {}], ['DELETE', `/bookings/${FAKE_ID}`],
  ['GET', '/stats/leaderboard'], ['GET', '/stats/imports'], ['GET', `/stats/imports/${FAKE_ID}`],
  ['POST', '/stats/imports', {}],
]
const playerRoutes = [
  ['GET', '/player/auth/me'],
  ['GET', '/player/games'], ['GET', '/player/games/X/slots'],
  ['GET', '/player/bookings'], ['POST', '/player/bookings', {}], ['GET', `/player/bookings/${FAKE_ID}`],
  ['GET', '/player/chats/players'], ['GET', `/player/chats/players/${A.id}`],
  ['GET', '/player/chats/conversations'],
  ['GET', `/player/chats/conversations/${A.id}/messages`],
  ['POST', `/player/chats/conversations/${A.id}/messages`, { body: 'x' }],
  ['POST', `/player/chats/conversations/${A.id}/read`],
  ['GET', '/player/stats'], ['GET', '/player/stats/games'], ['GET', `/player/stats/games/${FAKE_ID}`],
]

// A player token re-signed with a forged kind, and an unsigned "alg: none" token.
const [h, p] = A.token.split('.')
const forgedPayload = Buffer.from(JSON.stringify({ ...JSON.parse(Buffer.from(p, 'base64url')), kind: 'admin' })).toString('base64url')
const forged = `${h}.${forgedPayload}.${A.token.split('.')[2]}`
const algNone = `${Buffer.from('{"alg":"none","typ":"JWT"}').toString('base64url')}.${forgedPayload}.`

for (const [method, path, body] of adminRoutes) {
  const noAuth = await call(method, path, { body })
  const asPlayer = await call(method, path, { token: A.token, body })
  const tampered = await call(method, path, { token: forged, body })
  const none = await call(method, path, { token: algNone, body })
  check(noAuth.status === 401, `${method} ${path}: no token → ${noAuth.status}`)
  check(asPlayer.status === 403, `${method} ${path}: player token → ${asPlayer.status}`)
  check(tampered.status === 401 && none.status === 401, `${method} ${path}: forged tokens → ${tampered.status}/${none.status}`)
}
for (const [method, path, body] of playerRoutes) {
  const noAuth = await call(method, path, { body })
  const asAdmin = await call(method, path, { token: admin, body })
  const withCookie = await call(method, path, { cookie: adminCookie, body })
  check(noAuth.status === 401, `${method} ${path}: no token → ${noAuth.status}`)
  check(asAdmin.status === 403, `${method} ${path}: admin token → ${asAdmin.status}`)
  check(withCookie.status === 401, `${method} ${path}: admin cookie ignored → ${withCookie.status}`)
}
check(await wsHandshake() === 401, 'chat WS: no token → 401')
check(await wsHandshake('?token=garbage') === 401, 'chat WS: bad token → 401')
check(await wsHandshake(`?token=${admin}`) === 403, 'chat WS: admin token → 403')
check(await wsHandshake(`?token=${forged}`) === 401, 'chat WS: forged token → 401')
check(await wsHandshake('', { Authorization: `Bearer ${A.token}` }) === 101, 'chat WS: player token → 101')

// ---------------------------------------------------------------------------
// 2. Players can't see each other's data
// ---------------------------------------------------------------------------

const meA = await call('GET', '/player/auth/me', { token: A.token })

// Stats: a player's summary is their own; game records are only ever their own rows.
const statsA = await call('GET', '/player/stats', { token: A.token })
check(statsA.json?.player?.id === A.id, '/player/stats returns only the caller')
const gamesA = await call('GET', '/player/stats/games', { token: A.token })
check(gamesA.status === 200 && Array.isArray(gamesA.json.items), '/player/stats/games lists the caller\'s records')
const leaderboard = await call('GET', '/stats/leaderboard', { token: admin })
check(leaderboard.status === 200 && leaderboard.json.every((s) => !('email' in s.player) && !('phone' in s.player)), 'leaderboard exposes no contact details')
const anyGame = leaderboard.json.length ? await call('GET', '/stats/imports', { token: admin }) : null
const gameId = anyGame?.json?.find((i) => i.game)?.game?.id
if (gameId) {
  // A freshly created player played no games, so any existing game must 404 for them.
  const foreign = await call('GET', `/player/stats/games/${gameId}`, { token: A.token })
  check(foreign.status === 404, `player reading a game they didn't play → ${foreign.status}`)
}
check(meA.json?.id === A.id, '/player/auth/me returns only the caller')

const chatPlayers = await call('GET', `/player/chats/players?search=authz_&per_page=100`, { token: C.token })
const exposed = chatPlayers.json.items.flatMap(Object.keys)
check(!exposed.includes('email') && !exposed.includes('phone'), 'chat player list has no email/phone')
check(!chatPlayers.json.items.some((p) => p.id === C.id), 'chat player list excludes the caller')

const convC = await call('GET', '/player/chats/conversations', { token: C.token })
check(convC.json.length === 0, "C's conversation list doesn't include A↔B")
for (const other of [A, B]) {
  const hist = await call('GET', `/player/chats/conversations/${other.id}/messages`, { token: C.token })
  check(hist.json.items.length === 0, `C↔${other.playername.split('_')[1]} history doesn't expose A↔B messages`)
}
const cursor = await call('GET', `/player/chats/conversations/${A.id}/messages?before=${msg.json.id}`, { token: C.token })
check(cursor.json.items.length === 0, "C can't use an A↔B message id as a history cursor")
await call('POST', `/player/chats/conversations/${A.id}/read`, { token: C.token })
const convB = await call('GET', '/player/chats/conversations', { token: B.token })
check(convB.json.find((c) => c.player.id === A.id)?.unread_count === 1, "C marking 'read' doesn't touch B's unread A→B message")

// Live delivery goes only to the two participants
const sockC = await chatSocket(C.token)
await call('POST', `/player/chats/conversations/${B.id}/messages`, { token: A.token, body: { body: 'another private one' } })
await new Promise((r) => setTimeout(r, 400))
check(!sockC.events.some((e) => e.type === 'message'), 'C receives no live A→B messages')
sockC.ws.close()

if (WITH_BOOKINGS) {
  const other = await call('GET', `/player/bookings/${bookingA.id}`, { token: B.token })
  check(other.status === 404, `B reading A's booking → ${other.status}`)
  const listB = await call('GET', '/player/bookings', { token: B.token })
  check(listB.json.items.every((b) => b.id !== bookingA.id) && listB.json.items.some((b) => b.id === bookingB.id), "B's list has only B's bookings")
  const listA = await call('GET', '/player/bookings', { token: A.token })
  check(listA.json.items.every((b) => b.id !== bookingB.id), "A's list has only A's bookings")
  check(!('player' in listA.json.items[0]), 'player booking responses carry no player object')
}

// No secrets in any response we've seen
const bodies = [meA, chatPlayers, convC, convB, await call('GET', '/players', { token: admin }), await call('GET', '/bookings', { token: admin })]
check(bodies.every((b) => leaks(b.json).length === 0), `no response exposes ${SECRET_FIELDS.join('/')}`)

// ---------------------------------------------------------------------------
// 3. Sessions end when the account changes
// ---------------------------------------------------------------------------

const sockA = await chatSocket(A.token)
await call('PUT', `/players/${A.id}`, { token: admin, body: { full_name: 'Authz a', playername: A.playername, password: 'newpassword123' } })
check((await call('GET', '/player/auth/me', { token: A.token })).status === 401, "A's old token rejected after admin resets the password")
check(await sockA.waitFor(sockA.isClosed), "A's open chat socket closed after password reset")
check(await wsHandshake(`?token=${A.token}`) === 401, "A's old token can't reconnect the chat socket")
const relogin = await call('POST', '/player/auth/login', { body: { playername: A.playername, password: 'newpassword123' } })
check(relogin.status === 200 && (await call('GET', '/player/auth/me', { token: relogin.json.token })).status === 200, 'A can sign in again with the new password')

const sockB = await chatSocket(B.token)
await call('DELETE', `/players/${B.id}`, { token: admin })
created.splice(created.indexOf(B.id), 1)
check((await call('GET', '/player/bookings', { token: B.token })).status === 401, "deleted player's token rejected")
check(await sockB.waitFor(sockB.isClosed), "deleted player's chat socket closed")

// ---------------------------------------------------------------------------
// 4. Admin accounts, CSRF, throttling, caching
// ---------------------------------------------------------------------------

const signupBody = { username: `authz_admin_${suffix}`, name: 'x', password: 'password123' }
check((await call('POST', '/auth/signup', { body: signupBody })).status === 401, 'admin signup without auth → 401 (admins already exist)')
check((await call('POST', '/auth/signup', { token: C.token, body: signupBody })).status === 401, 'admin signup with a player token → 401')

const evil = await call('DELETE', `/players/${C.id}`, { cookie: adminCookie, headers: { Origin: 'https://evil.example' } })
check(evil.status === 403, `cross-site DELETE with the admin cookie → ${evil.status}`)
check((await call('GET', `/players/${C.id}`, { token: admin })).status === 200, '...and the player was not deleted')

const ip = '203.0.113.77'
const badLogin = () => call('POST', '/player/auth/login', { headers: { 'X-Real-IP': ip }, body: { playername: C.playername, password: 'wrong-password' } })
const statuses = []
for (let i = 0; i < 6; i++) statuses.push((await badLogin()).status)
check(statuses.slice(0, 5).every((s) => s === 401) && statuses[5] === 429, `player login locks after 5 failures: ${statuses.join(',')}`)
const lockedGood = await call('POST', '/player/auth/login', { headers: { 'X-Real-IP': ip }, body: { playername: C.playername, password: 'password123' } })
check(lockedGood.status === 429, 'correct password is also refused while locked out')
const otherIp = await call('POST', '/player/auth/login', { body: { playername: C.playername, password: 'password123' } })
check(otherIp.status === 200, 'another client can still sign in')
const unknown = await call('POST', '/player/auth/login', { body: { playername: `nobody_${suffix}`, password: 'x' } })
check(unknown.status === 401 && unknown.json?.error === 'Invalid playername or password', 'unknown playername gets the same error as a wrong password')

check(meA.headers.get('cache-control') === 'no-store', `API responses are not cacheable (${meA.headers.get('cache-control')})`)

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

for (const id of created) await call('DELETE', `/players/${id}`, { token: admin })

console.log(`\n${passed} passed, ${failures.length} failed`)
if (failures.length) {
  for (const f of failures) console.log(`  ✗ ${f}`)
  process.exit(1)
}
process.exit(0)
