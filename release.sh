#!/usr/bin/env bash
# Tests, builds in release mode and packages the app into dist/.
# The same script runs locally and in CI. It never commits, tags or uploads.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

NAME="github-desktop-enhanced"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
PACKAGE="${NAME}-${VERSION}-${OS}-${ARCH}"

echo "==> Checking formatting and lints"
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings

echo "==> Running tests"
cargo test --locked

echo "==> Building ${PACKAGE}"
cargo build --locked --release

echo "==> Packaging"
# Only this version's package is kept, so an older install.sh can't be run by mistake.
rm -rf dist/"${NAME}"-*
mkdir -p "dist/${PACKAGE}"
cp "target/release/${NAME}" "dist/${PACKAGE}/"
cp packaging/${NAME}.desktop packaging/${NAME}.svg packaging/install.sh "dist/${PACKAGE}/"
cp README.md "dist/${PACKAGE}/"
chmod +x "dist/${PACKAGE}/install.sh"
tar -C dist -czf "dist/${PACKAGE}.tar.gz" "${PACKAGE}"

echo "==> Created dist/${PACKAGE}.tar.gz"
