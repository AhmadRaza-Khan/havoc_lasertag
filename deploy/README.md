# Deployment

Both apps run on one VPS behind nginx:

| Domain | Serves | From |
|---|---|---|
| `havoc-admin.cyberboost.io` | React admin panel (static files) | `/var/www/havoc/current` |
| `havoc-api.cyberboost.io` | Rust API + chat WebSocket | `127.0.0.1:8080` (systemd `havoc-api`) |

**Everything is built on GitHub.** The VPS only receives the compiled API binary (static, no
Rust toolchain or system libraries needed) and the React `dist/` files — no source code.

```
push to main ─► CI (fmt, clippy, tests, lint) ─► build API binary ─┐
                                              └► build web app ────┴► upload to VPS ─► switch API
                                                                     ─► health check ─► switch web
```

Each deploy is a new folder under `releases/`, activated by switching a `current` symlink.
If the API fails its health check, the previous API release is restored automatically and the
web app is not switched. The last 5 releases are kept for the **Rollback** workflow.
Database migrations are inside the binary and run when it starts.

Production: VPS `13.140.177.43` (Ubuntu 24.04) with the domains above. To use other domains,
change them in `deploy/nginx/*.conf`, `client/.env.production` and the workflow defaults.

---

## 1. GitHub settings

**Settings → Secrets and variables → Actions → Secrets**

| Secret | Required | Value |
|---|---|---|
| `VPS_HOST` | yes | VPS IP or hostname, e.g. `203.0.113.10` |
| `VPS_USER` | yes | SSH user the workflow logs in as: `deploy` (created below) |
| `VPS_SSH_KEY` | yes | **Private** key of the deploy key pair (whole file, including the `BEGIN`/`END` lines) |
| `VPS_KNOWN_HOSTS` | yes | Output of `ssh-keyscan -p 22 <VPS_HOST>` — pins the server's identity |
| `VPS_PORT` | no | SSH port if not `22` |

**Settings → Secrets and variables → Actions → Variables** (all optional; defaults shown)

| Variable | Default | Used for |
|---|---|---|
| `API_URL` | `https://havoc-api.cyberboost.io` | API address baked into the web app |
| `WEB_URL` | `https://havoc-admin.cyberboost.io` | Link shown on the deployment |
| `BUSINESS_TIMEZONE` | `Asia/Qatar` | Timezone the web app shows bookings in |
| `API_ROOT` | `/opt/havoc` | API releases folder on the VPS |
| `WEB_ROOT` | `/var/www/havoc` | Web releases folder on the VPS |
| `HEALTH_URL` | `http://127.0.0.1:8080/api/health` | Checked on the VPS after each deploy |

**Settings → Environments → New environment → `production`.** Optionally add required
reviewers so each deploy waits for approval.

Create the deploy key pair on your own machine (not the VPS):

```bash
ssh-keygen -t ed25519 -C "github-deploy" -f havoc_deploy -N ""
# havoc_deploy      → paste into the VPS_SSH_KEY secret
# havoc_deploy.pub  → goes into the VPS (step 2.3)
ssh-keyscan -p 22 <VPS_HOST>   # → paste into the VPS_KNOWN_HOSTS secret
```

---

## 2. One-time VPS setup (Ubuntu/Debian)

### 2.1 Packages and DNS

Point `A` (and `AAAA` if you use IPv6) records for `havoc-admin.cyberboost.io` and `havoc-api.cyberboost.io` at the VPS. Then:

```bash
sudo apt update
sudo apt install -y nginx postgresql certbot python3-certbot-nginx curl ca-certificates
```

### 2.2 Database

```bash
sudo -u postgres psql <<'SQL'
CREATE USER havoc WITH PASSWORD 'CHANGE-ME-STRONG-PASSWORD';
CREATE DATABASE havoc OWNER havoc;
SQL
```

### 2.3 Users and folders

```bash
# Runs the API; no login, no home.
sudo adduser --system --group --no-create-home havoc

# Used by GitHub Actions over SSH.
sudo adduser --disabled-password --gecos "" deploy
sudo mkdir -p /home/deploy/.ssh
sudo cp havoc_deploy.pub /home/deploy/.ssh/authorized_keys   # the PUBLIC key
sudo chown -R deploy:deploy /home/deploy/.ssh
sudo chmod 700 /home/deploy/.ssh && sudo chmod 600 /home/deploy/.ssh/authorized_keys

# Release folders: written by `deploy`, readable by the API user and nginx.
sudo mkdir -p /opt/havoc/releases /opt/havoc/shared /var/www/havoc/releases
sudo chown deploy:deploy /opt/havoc /opt/havoc/releases /var/www/havoc /var/www/havoc/releases
sudo chown root:havoc /opt/havoc/shared && sudo chmod 750 /opt/havoc/shared
```

`deploy` may restart the API and read its logs, nothing else:

```bash
echo 'deploy ALL=(root) NOPASSWD: /usr/bin/systemctl restart havoc-api, /usr/bin/journalctl -u havoc-api *' \
  | sudo tee /etc/sudoers.d/havoc-deploy
sudo chmod 440 /etc/sudoers.d/havoc-deploy
sudo visudo -c
```

### 2.4 API configuration

Create `/opt/havoc/shared/.env` (see `server/.env.example`):

```bash
sudo tee /opt/havoc/shared/.env >/dev/null <<'EOF'
DATABASE_URL=postgres://havoc:CHANGE-ME-STRONG-PASSWORD@localhost:5432/havoc
BIND_ADDR=127.0.0.1:8080
JWT_SECRET=REPLACE-WITH-OUTPUT-OF-openssl-rand-base64-48
JWT_TTL_HOURS=168
CORS_ORIGINS=https://havoc-admin.cyberboost.io
COOKIE_SECURE=true
BOOKEO_API_KEY=...
BOOKEO_SECRET_KEY=...
BOOKEO_TIMEZONE=Asia/Qatar
RUST_LOG=server=info,tower_http=info
EOF
sudo chown root:havoc /opt/havoc/shared/.env
sudo chmod 640 /opt/havoc/shared/.env
```

Generate the JWT secret with `openssl rand -base64 48`. Use a new one for production, not the
development value.

### 2.5 systemd service

```bash
sudo cp deploy/systemd/havoc-api.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable havoc-api    # it starts on the first deploy
```

### 2.6 HTTPS and nginx

One certificate (named `havoc`) covers both names:

```bash
sudo certbot certonly --nginx --cert-name havoc -d havoc-admin.cyberboost.io -d havoc-api.cyberboost.io
sudo cp deploy/nginx/havoc-admin.cyberboost.io.conf deploy/nginx/havoc-api.cyberboost.io.conf /etc/nginx/sites-available/
sudo ln -s /etc/nginx/sites-available/havoc-admin.cyberboost.io.conf /etc/nginx/sites-enabled/
sudo ln -s /etc/nginx/sites-available/havoc-api.cyberboost.io.conf /etc/nginx/sites-enabled/
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t && sudo systemctl reload nginx
```

Certbot renews automatically. Until the first deploy, `havoc-admin.cyberboost.io` answers 404 — expected.

---

### 2.7 Notes for game statistics uploads

Nothing to install: OCR runs inside the API binary (its models are embedded at build time; the
first CI build downloads them, ~12 MB, and checksums them). nginx already allows 30 MB uploads
on `/api/stats/imports`. Uploads are processed one at a time and use all CPU cores for
~30 seconds per game.

## 3. Deploying

- **Automatic:** every push to `main` runs CI, builds and deploys.
- **Manual:** Actions → **Deploy** → Run workflow.
- **Rollback:** Actions → **Rollback** → Run workflow with `previous` (the release before the
  live one), a release name from a deploy summary, or `list` to see what's on the server.

Migrations must stay backward compatible (add columns/tables; don't drop or rename in the
same release), so the previous binary still works if you roll back.

### First admin user

Signup is blocked at nginx, and the API only allows it without a token while no admin exists.
Create the first admin on the VPS itself:

```bash
curl -X POST http://127.0.0.1:8080/api/auth/signup \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","name":"Admin","password":"a-strong-password"}'
```

Further admins need an existing admin's token:
`-H "Authorization: Bearer <token from /api/auth/login>"` on the same request.

### Useful commands on the VPS

```bash
sudo systemctl status havoc-api
sudo journalctl -u havoc-api -f
ls -l /opt/havoc/current /var/www/havoc/current   # live release
```
