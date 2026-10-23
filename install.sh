#!/bin/sh
# install.sh — install the dockit CLI from its public GitHub releases.
#
#   curl -fsSL https://raw.githubusercontent.com/raeffs/tug/main/install.sh | sh
#
# Installs the latest stable release by default, or a pinned version. Verifies
# the published sha256 checksum before installing. dockit ships as a single static
# x86_64-unknown-linux-musl binary, so it runs on any x86_64 Linux (glibc or
# musl), no runtime dependencies.
#
# Configuration (environment variables, or the flags shown in --help):
#   DOCKIT_VERSION       pin a release, e.g. v0.1.0   (default: latest stable)
#   DOCKIT_INSTALL_DIR   install location             (default: ~/.local/bin)
set -eu

REPO="raeffs/tug"
TARGET="x86_64-unknown-linux-musl"

VERSION="${DOCKIT_VERSION:-${TUG_VERSION:-}}"
INSTALL_DIR="${DOCKIT_INSTALL_DIR:-${TUG_INSTALL_DIR:-${HOME}/.local/bin}}"

log()  { printf 'install.sh: %s\n' "$*" >&2; }
die()  { printf 'install.sh: error: %s\n' "$*" >&2; exit 1; }

usage() {
  cat >&2 <<EOF
Usage: install.sh [options] [VERSION]

Install the dockit CLI from https://github.com/${REPO}/releases.

Options:
  -v, --version VERSION   install VERSION (e.g. v0.1.0); default is latest stable
  -d, --dir DIR           install into DIR; default ~/.local/bin
  -h, --help              show this help

A bare VERSION argument is equivalent to --version. Environment variables
DOCKIT_VERSION and DOCKIT_INSTALL_DIR provide the same settings; explicit flags win.

Examples:
  curl -fsSL .../install.sh | sh
  curl -fsSL .../install.sh | sh -s -- v0.1.0
  DOCKIT_VERSION=v0.1.0 DOCKIT_INSTALL_DIR=/usr/local/bin sh install.sh
EOF
}

# --- parse arguments ---------------------------------------------------------
while [ "$#" -gt 0 ]; do
  case "$1" in
    -v|--version) [ "$#" -ge 2 ] || die "$1 needs a value"; VERSION="$2"; shift 2 ;;
    -d|--dir)     [ "$#" -ge 2 ] || die "$1 needs a value"; INSTALL_DIR="$2"; shift 2 ;;
    -h|--help)    usage; exit 0 ;;
    -*)           usage; die "unknown option: $1" ;;
    *)            VERSION="$1"; shift ;;
  esac
done

# --- preflight ---------------------------------------------------------------
command -v curl >/dev/null 2>&1 || die "curl is required"

os="$(uname -s)"
arch="$(uname -m)"
[ "$os" = "Linux" ] || die "unsupported OS '$os' (only Linux x86_64 is published)"
case "$arch" in
  x86_64|amd64) : ;;
  *) die "unsupported architecture '$arch' (only x86_64 is published)" ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
  checksum_cmd="sha256sum -c"
elif command -v shasum >/dev/null 2>&1; then
  checksum_cmd="shasum -a 256 -c"
else
  die "need sha256sum or shasum to verify the download"
fi

# --- resolve version ---------------------------------------------------------
# With no pinned version, follow the /releases/latest redirect to discover the
# newest stable tag. This avoids the rate-limited GitHub API and needs no jq.
if [ -z "$VERSION" ]; then
  log "resolving latest stable release..."
  effective="$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
                 "https://github.com/${REPO}/releases/latest")" \
    || die "could not reach GitHub to resolve the latest release"
  VERSION="${effective##*/}"
  case "$effective" in
    */releases/tag/*) : ;;
    *) die "no stable release found for ${REPO} (only pre-releases exist? pin one with --version)" ;;
  esac
fi

case "$VERSION" in
  v*) : ;;
  *)  die "version must look like vX.Y.Z (got '$VERSION')" ;;
esac

ASSET="dockit-${VERSION}-${TARGET}"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

# --- download + verify -------------------------------------------------------
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

log "downloading ${ASSET} (${VERSION})..."
curl -fSL --proto '=https' --tlsv1.2 -o "${tmp}/${ASSET}" "${BASE_URL}/${ASSET}" \
  || die "download failed — does release ${VERSION} exist?"
curl -fSL --proto '=https' --tlsv1.2 -o "${tmp}/${ASSET}.sha256" "${BASE_URL}/${ASSET}.sha256" \
  || die "checksum download failed for ${VERSION}"

log "verifying checksum..."
# The .sha256 names the bare asset, so verify from inside the temp dir.
( cd "$tmp" && $checksum_cmd "${ASSET}.sha256" ) >/dev/null \
  || die "checksum verification FAILED — refusing to install"

# --- install -----------------------------------------------------------------
mkdir -p "$INSTALL_DIR" || die "cannot create install dir: $INSTALL_DIR"
cp "${tmp}/${ASSET}" "${INSTALL_DIR}/dockit" || die "cannot write to $INSTALL_DIR"
chmod 0755 "${INSTALL_DIR}/dockit"

log "installed dockit ${VERSION} -> ${INSTALL_DIR}/dockit"
"${INSTALL_DIR}/dockit" --version >/dev/null 2>&1 \
  && log "$("${INSTALL_DIR}/dockit" --version 2>&1)"

# PATH hint (POSIX-safe membership test).
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) : ;;
  *) log "note: ${INSTALL_DIR} is not on your PATH — add it, e.g.:"
     log "  export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
esac
