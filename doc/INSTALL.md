# Hitide Installation

Hitide is the browser frontend for Lotide. It renders HTML and CSS and talks to
the Lotide backend through `BACKEND_HOST`. It does not own the database,
ActivityPub inboxes, migrations, media storage, or background tasks.

This guide documents the direct Debian source-tree install used by the live
production instance. There are also helper scripts under `build_scripts/` that install
a conventional `/usr/local/bin` layout, but the direct `/var/hitide` layout is
the one described here because it matches the wrapper script and systemd unit
that are known to start the current service successfully.

Do not copy live passwords from another instance. The examples below use
placeholders.

## Relationship To Lotide

Install and start Lotide first. Hitide needs:

- a reachable Lotide backend, normally `http://127.0.0.1:3333`
- the public frontend URL, normally `https://LOTIDE_DOMAIN`
- the reverse proxy route that sends normal browser traffic to Hitide

Hitide listens on `127.0.0.1:4333` by default. If the reverse proxy runs on
another host, set `BIND_ADDRESS` to a reachable private address such as
`0.0.0.0` or the server's LAN address, then firewall the port so only the proxy
can connect.

## Debian Packages

Install the build and runtime packages:

```sh
sudo apt update
sudo apt install -y \
  build-essential \
  ca-certificates \
  curl \
  git \
  libssl-dev \
  openssl \
  pkg-config
```

Use a current Rust toolchain. The project uses Rust 2024, so `rustup` stable is
preferred over old distro Rust packages:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
rustup toolchain install stable
rustup component add clippy rustfmt
```

## System User

The live deployment runs both Lotide and Hitide as the same `lotide` user:

```sh
sudo adduser --system --group --home /var/lotide --shell /usr/sbin/nologin lotide
```

If you already installed Lotide, this user should already exist.

## Source Checkout

Clone the frontend into `/var/hitide`:

```sh
cd /var
sudo git clone https://git.sr.ht/~vpzom/hitide/ hitide
sudo chown -R lotide:lotide /var/hitide
```

If you are deploying this locally maintained tree instead of the public upstream,
copy or clone that tree into `/var/hitide` and keep the same ownership.

## Build

Build the release binary:

```sh
cd /var/hitide
cargo build --release --bin hitide
sudo chown -R lotide:lotide /var/hitide
```

The binary should exist at:

```sh
/var/hitide/target/release/hitide
```

## Wrapper Script

Create `/var/hitide/target/release/hitide.sh`:

```sh
#!/bin/bash

export DATABASE_URL="postgresql://lotide:LOTIDE_DB_PASSWORD@localhost:5432/lotide"
export HOST_URL_ACTIVITYPUB="https://LOTIDE_DOMAIN/apub"
export HOST_URL_API="https://LOTIDE_DOMAIN/api"
export FRONTEND_URL="https://LOTIDE_DOMAIN"
export BIND_ADDRESS="127.0.0.1"

export APUB_PROXY_REWRITES=true
export ALLOW_FORWARDED=true
export BACKEND_HOST="http://127.0.0.1:3333"
export MEDIA_LOCATION="/var/lotide/media/"

# Optional mail configuration. Hitide does not use these directly.
# export SMTP_URL="smtps://SMTP_USERNAME:SMTP_PASSWORD@SMTP_SERVER"
# export SMTP_FROM="SMTP_FROM"

cd /var/hitide/target/release
exec ./hitide
```

Then set ownership and permissions:

```sh
sudo chown lotide:lotide /var/hitide/target/release/hitide.sh
sudo chmod 0750 /var/hitide/target/release/hitide.sh
```

The working live wrapper keeps a shared Lotide environment block for operational
symmetry. Hitide's current configuration reads `BACKEND_HOST`, `FRONTEND_URL`,
`BIND_ADDRESS`, and optional `PORT`, so the database, ActivityPub, media, and
SMTP values are harmless but not required by Hitide.

The minimal equivalent is:

```sh
#!/bin/bash

export BACKEND_HOST="http://127.0.0.1:3333"
export FRONTEND_URL="https://LOTIDE_DOMAIN"
export BIND_ADDRESS="127.0.0.1"

cd /var/hitide/target/release
exec ./hitide
```

If your HTTPS reverse proxy is not on the Hitide host, change the wrapper to
bind the frontend to an address the proxy can reach:

```sh
export BIND_ADDRESS="0.0.0.0"
```

Use a specific private interface address instead of `0.0.0.0` when that is
cleaner for your network. `FRONTEND_URL` should still be the public HTTPS URL.

## Systemd

Create `/etc/systemd/system/hitide.service`:

```ini
[Unit]
Description=Hitide
After=network.target lotide.service
Wants=lotide.service

[Service]
Type=simple
User=lotide
Group=lotide
ExecStart=/var/hitide/target/release/hitide.sh
Restart=on-failure
RestartSec=10
KillMode=control-group
TimeoutStopSec=30

[Install]
WantedBy=multi-user.target
```

Load the unit and start Hitide:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now hitide.service
sudo systemctl status hitide.service
```

Logs:

```sh
sudo journalctl -u hitide -f
```

## Reverse Proxy

Hitide should sit behind the same public host as Lotide. Normal browser traffic
goes to Hitide on port `4333`; API, ActivityPub, and WebFinger routes go to
Lotide on port `3333`.

The Lotide install guide contains the full Nginx example. The Hitide-specific
part is:

```nginx
location / {
    proxy_pass http://127.0.0.1:4333;
}
```

Keep the ActivityPub content negotiation rewrite from the Lotide guide so that
ActivityPub requests for user, community, and post URLs are routed to the
backend instead of the browser frontend.

## Configuration Reference

Required Hitide settings:

- `BACKEND_HOST`: URL Hitide uses to reach Lotide, for example
  `http://127.0.0.1:3333`.
- `FRONTEND_URL`: public URL for the site, for example
  `https://LOTIDE_DOMAIN`.

Optional settings:

- `BIND_ADDRESS`: frontend listen address. Defaults to `127.0.0.1`. Use
  `0.0.0.0`, `::`, or a private interface address when a reverse proxy on
  another host needs direct access.
- `PORT`: frontend listen port. Defaults to `4333`.
- `RUST_LOG`: optional log filter, for example `hitide=info`.

Hitide also accepts variables with a `HITIDE_` prefix. For example,
`BACKEND_HOST` and `HITIDE_BACKEND_HOST` both map to the same field.

## Updating

For a direct source-tree install:

```sh
sudo systemctl stop hitide.service
cd /var/hitide
git pull
cargo build --release --bin hitide
sudo chown -R lotide:lotide /var/hitide
sudo systemctl start hitide.service
```

If you update both backend and frontend, stop Hitide first, then Lotide. Start
Lotide first, then Hitide.

/* end of INSTALL.md */
