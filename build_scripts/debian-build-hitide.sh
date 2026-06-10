#!/usr/bin/env bash
#
# Project: Hitide Debian Build Support
# ------------------------------------
#
# File: debian-build-hitide.sh
#
# Purpose:
#
#     Install the Debian packages needed to compile Hitide, ensure a modern
#     Rust toolchain is available, and build the release binary.
#
# Responsibilities:
#
#     - install native Debian build dependencies
#     - install Rust through rustup when cargo is not already available
#     - build the Hitide release binary with cargo
#
# This file intentionally does NOT contain:
#
#     - backend Lotide configuration
#     - reverse proxy configuration
#     - systemd service installation

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

APT_PACKAGES=(
    build-essential
    ca-certificates
    curl
    git
    libssl-dev
    openssl
    pkg-config
)

run_apt_get() {
    if [ "$(id -u)" -eq 0 ]; then
        DEBIAN_FRONTEND=noninteractive apt-get "$@"
    elif command -v sudo >/dev/null 2>&1; then
        sudo env DEBIAN_FRONTEND=noninteractive apt-get "$@"
    else
        echo "This script needs root or sudo to install Debian packages." >&2
        exit 1
    fi
}

ensure_debian_packages() {
    if [ ! -r /etc/debian_version ]; then
        echo "This script is intended for Debian or Debian-derived systems." >&2
        exit 1
    fi

    run_apt_get update
    run_apt_get install -y "${APT_PACKAGES[@]}"
}

ensure_rust_toolchain() {
    if ! command -v rustup >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
            sh -s -- -y --default-toolchain stable
    fi

    # shellcheck disable=SC1091
    if [ -f "${HOME}/.cargo/env" ]; then
        . "${HOME}/.cargo/env"
    fi

    rustup toolchain install stable
    rustup component add clippy rustfmt

    if ! command -v cargo >/dev/null 2>&1; then
        echo "cargo is still unavailable after installing Rust." >&2
        exit 1
    fi
}

configure_cargo_resource_limits() {
    # Production Hitide hosts are often the same small VPS as Lotide. Rust can
    # use several compiler processes at once and debug information makes the
    # final link much larger, so keep the default build envelope conservative.
    # Larger build machines can still override these values in the environment.
    export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
    export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"

    if [ -z "${RUSTFLAGS:-}" ]; then
        export RUSTFLAGS="-C debuginfo=0"
    fi
}

run_cargo_build() {
    if command -v ionice >/dev/null 2>&1; then
        nice -n 10 ionice -c 2 -n 7 cargo "$@"
    else
        nice -n 10 cargo "$@"
    fi
}

build_hitide() {
    cd "${PROJECT_DIR}"
    run_cargo_build build --release --bin hitide -j "${CARGO_BUILD_JOBS}"
}

ensure_debian_packages
ensure_rust_toolchain
configure_cargo_resource_limits
build_hitide

echo "Hitide built at ${PROJECT_DIR}/target/release/hitide"

# end of debian-build-hitide.sh
