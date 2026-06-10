#!/usr/bin/env bash
#
# Project: Hitide Debian Install Support
# --------------------------------------
#
# File: debian-install-hitide.sh
#
# Purpose:
#
#     Install a previously built Hitide binary onto a Debian host.
#
# Responsibilities:
#
#     - install runtime packages
#     - create the hitide system user and state directory
#     - install a default environment file without overwriting local settings
#     - install a systemd unit for the frontend service
#
# This file intentionally does NOT contain:
#
#     - Lotide backend installation
#     - PostgreSQL setup
#     - reverse proxy configuration

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

PREFIX="${PREFIX:-/usr/local}"
BIN_DIR="${BIN_DIR:-${PREFIX}/bin}"
HITIDE_USER="${HITIDE_USER:-hitide}"
HITIDE_GROUP="${HITIDE_GROUP:-${HITIDE_USER}}"
HITIDE_CONFIG_DIR="${HITIDE_CONFIG_DIR:-/etc/hitide}"
HITIDE_ENV_FILE="${HITIDE_ENV_FILE:-${HITIDE_CONFIG_DIR}/hitide.env}"
HITIDE_STATE_DIR="${HITIDE_STATE_DIR:-/var/lib/hitide}"
HITIDE_BINARY="${HITIDE_BINARY:-${PROJECT_DIR}/target/release/hitide}"

APT_PACKAGES=(
    ca-certificates
    openssl
)

require_root() {
    if [ "$(id -u)" -ne 0 ]; then
        echo "Run this install script as root." >&2
        exit 1
    fi
}

install_runtime_packages() {
    if [ ! -r /etc/debian_version ]; then
        echo "This script is intended for Debian or Debian-derived systems." >&2
        exit 1
    fi

    DEBIAN_FRONTEND=noninteractive apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y "${APT_PACKAGES[@]}"
}

install_hitide_user() {
    if ! getent group "${HITIDE_GROUP}" >/dev/null; then
        groupadd --system "${HITIDE_GROUP}"
    fi

    if ! id "${HITIDE_USER}" >/dev/null 2>&1; then
        useradd \
            --system \
            --gid "${HITIDE_GROUP}" \
            --home-dir "${HITIDE_STATE_DIR}" \
            --shell /usr/sbin/nologin \
            "${HITIDE_USER}"
    fi

    install -d -o "${HITIDE_USER}" -g "${HITIDE_GROUP}" -m 0750 "${HITIDE_STATE_DIR}"
    install -d -o root -g root -m 0755 "${HITIDE_CONFIG_DIR}"
}

install_hitide_binary() {
    if [ ! -x "${HITIDE_BINARY}" ]; then
        echo "Missing built binary: ${HITIDE_BINARY}" >&2
        echo "Run build_scripts/debian-build-hitide.sh first." >&2
        exit 1
    fi

    install -d -o root -g root -m 0755 "${BIN_DIR}"
    install -o root -g root -m 0755 "${HITIDE_BINARY}" "${BIN_DIR}/hitide"
}

install_hitide_environment() {
    if [ -e "${HITIDE_ENV_FILE}" ]; then
        echo "Keeping existing ${HITIDE_ENV_FILE}"
        return
    fi

    cat >"${HITIDE_ENV_FILE}" <<EOF
# Hitide environment file.
# Edit these values before starting the service.

BACKEND_HOST=http://127.0.0.1:3333
FRONTEND_URL=https://example.com
BIND_ADDRESS=127.0.0.1
PORT=4333
RUST_LOG=hitide=info
EOF

    chown root:"${HITIDE_GROUP}" "${HITIDE_ENV_FILE}"
    chmod 0640 "${HITIDE_ENV_FILE}"
}

install_systemd_unit() {
    cat >/etc/systemd/system/hitide.service <<EOF
[Unit]
Description=Hitide Lotide frontend
After=network-online.target lotide.service
Wants=network-online.target

[Service]
Type=simple
User=${HITIDE_USER}
Group=${HITIDE_GROUP}
WorkingDirectory=${HITIDE_STATE_DIR}
EnvironmentFile=${HITIDE_ENV_FILE}
ExecStart=${BIN_DIR}/hitide
Restart=on-failure
RestartSec=5
NoNewPrivileges=true
PrivateTmp=true
ProtectHome=true
ProtectSystem=full

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
}

require_root
install_runtime_packages
install_hitide_user
install_hitide_binary
install_hitide_environment
install_systemd_unit

cat <<EOF
Hitide has been installed.

Next steps:
  1. Edit ${HITIDE_ENV_FILE}.
  2. Make sure Lotide is reachable at BACKEND_HOST.
  3. Start the service:
       systemctl enable --now hitide.service
EOF

# end of debian-install-hitide.sh
