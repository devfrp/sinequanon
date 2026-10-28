#!/usr/bin/env bash
# Installation de Sine qua non (Rust) depuis les versions GitHub.
#
# Utilisation :
#   curl -fsSL https://raw.githubusercontent.com/devfrp/sinequanon/main/install.sh | bash
#
# Options :
#   --version v0.1.0   installer une version précise
#   --bin-dir /chemin  répertoire d'installation (défaut : /usr/local/bin ou ~/.local/bin)
#
# Dépôt public : aucun token requis. Pour un miroir privé, exportez
# GITHUB_TOKEN (ou connectez `gh login`).

set -euo pipefail

REPO="devfrp/sinequanon"
PROG="sinequanon"
VERSION=""
BIN_DIR=""

usage() {
    sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
}

die() { echo "erreur : $*" >&2; exit 1; }
info() { echo "==> $*"; }

while [ $# -gt 0 ]; do
    case "$1" in
        --version) [ $# -ge 2 ] || die "--version attend une valeur"; VERSION="$2"; shift 2 ;;
        --bin-dir) [ $# -ge 2 ] || die "--bin-dir attend une valeur"; BIN_DIR="$2"; shift 2 ;;
        -h|--help|aide) usage ;;
        *) die "option inconnue : $1 (voir --help)" ;;
    esac
done

case "$(uname -s)" in
    Linux) OS="linux" ;;
    Darwin) OS="macos" ;;
    MINGW*|MSYS*|CYGWIN*)
        die "Windows : téléchargez directement sinequanon-windows-x86_64.exe sur
https://github.com/${REPO}/releases" ;;
    *) die "système non pris en charge : $(uname -s)" ;;
esac

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) die "architecture non prise en charge : $ARCH" ;;
esac

# --- téléchargement -----------------------------------------------------------

if command -v curl >/dev/null 2>&1; then
    fetch() { curl -fsSL "$1" -o "$2"; }
    fetch_api() {
        if [ -n "${GITHUB_TOKEN:-}" ]; then
            curl -fsSL -H "Authorization: Bearer ${GITHUB_TOKEN}" \
                 -H "Accept: application/vnd.github+json" "$1" -o "$2"
        else
            curl -fsSL "$1" -o "$2"
        fi
    }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -qO "$2" "$1"; }
    fetch_api() {
        if [ -n "${GITHUB_TOKEN:-}" ]; then
            wget -qO "$2" --header="Authorization: Bearer ${GITHUB_TOKEN}" "$1"
        else
            wget -qO "$2" "$1"
        fi
    }
else
    die "curl ou wget est requis"
fi

# gh connecté → téléchargements via gh : gère les dépôts privés ET les
# releases encore en brouillon (l'URL publique releases/download/ ne sert
# pas les assets d'un brouillon, même avec un token).
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    USE_GH=1
else
    USE_GH=0
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [ -z "$VERSION" ]; then
    info "Recherche de la dernière version…"
    if [ "$USE_GH" = 1 ]; then
        VERSION="$(gh api "repos/${REPO}/releases/latest" --jq .tag_name 2>/dev/null || true)"
    else
        fetch_api "https://api.github.com/repos/${REPO}/releases/latest" "$TMP/release.json" || true
        VERSION="$(grep -m1 '"tag_name"' "$TMP/release.json" 2>/dev/null | cut -d'"' -f4 || true)"
    fi
    [ -n "$VERSION" ] || die "impossible de déterminer la dernière version
(dépôt privé (miroir) ? exportez GITHUB_TOKEN, ou lancez : gh auth login)"
fi
info "Version : ${VERSION}"

if [ "$OS" = "macos" ]; then
    CANDIDATES="${PROG}-macos-universal.tar.gz"
else
    # binaire statique musl en priorité, fallback glibc
    CANDIDATES="${PROG}-${OS}-${ARCH}-musl.tar.gz ${PROG}-${OS}-${ARCH}-gnu.tar.gz"
fi

BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
download_asset() {
    # $1 = nom de l'asset, $2 = fichier de destination
    if [ "$USE_GH" = 1 ]; then
        gh release download "$VERSION" --repo "$REPO" --pattern "$1" --dir "$TMP" >/dev/null 2>&1 || return 1
        [ "$2" = "$TMP/$1" ] || mv "$TMP/$1" "$2" || return 1
    else
        fetch_api "${BASE_URL}/$1" "$2" || return 1
    fi
}

ASSET=""
for c in $CANDIDATES; do
    info "Essai de ${c}…"
    if download_asset "$c" "$TMP/asset"; then
        ASSET="$c"
        break
    fi
done
[ -n "$ASSET" ] || die "aucun fichier pour ${OS}/${ARCH} dans ${VERSION}
(assets attendus : ${CANDIDATES})"

# --- somme de contrôle --------------------------------------------------------

if download_asset "SHA256SUMS" "$TMP/SHA256SUMS"; then
    EXPECTED="$(grep " ${ASSET}\$" "$TMP/SHA256SUMS" | awk '{print $1}' | head -1 || true)"
    if [ -n "$EXPECTED" ]; then
        ACTUAL="$(sha256sum "$TMP/asset" 2>/dev/null | awk '{print $1}' \
            || shasum -a 256 "$TMP/asset" | awk '{print $1}' \
            || openssl dgst -sha256 "$TMP/asset" | awk '{print $NF}')"
        if [ -n "$ACTUAL" ] && [ "$ACTUAL" != "$EXPECTED" ]; then
            die "somme SHA-256 incorrecte (${ASSET})"
        fi
        info "Somme SHA-256 vérifiée."
    fi
fi

# --- installation --------------------------------------------------------------

tar -xzf "$TMP/asset" -C "$TMP"
[ -f "$TMP/${PROG}" ] || [ -f "$TMP/${PROG}.exe" ] || die "archive inattendue"
chmod +x "$TMP/${PROG}" 2>/dev/null || chmod +x "$TMP/${PROG}.exe"

if [ -z "$BIN_DIR" ]; then
    if [ -w /usr/local/bin ] 2>/dev/null || { command -v sudo >/dev/null 2>&1 && sudo -n true 2>/dev/null; }; then
        BIN_DIR="/usr/local/bin"
    else
        BIN_DIR="${HOME}/.local/bin"
    fi
fi

mkdir -p "$BIN_DIR" 2>/dev/null || die "impossible de créer ${BIN_DIR}"
if [ -w "$BIN_DIR" ]; then
    cp "$TMP/${PROG}" "${BIN_DIR}/${PROG}"
else
    sudo cp "$TMP/${PROG}" "${BIN_DIR}/${PROG}"
fi
info "Installé dans ${BIN_DIR}/${PROG}"

case ":${PATH}:" in
    *":${BIN_DIR}:"*) ;;
    *)
        echo ""
        echo "Ajoutez ${BIN_DIR} à votre PATH si nécessaire :"
        echo "  echo 'export PATH=\"${BIN_DIR}:\$PATH\"' >> ~/.bashrc   # ou ~/.zshrc"
        ;;
esac

echo ""
"${BIN_DIR}/${PROG}" --version 2>/dev/null && echo "Installation réussie ✔" \
    || echo "Installation terminée. Lancez : ${BIN_DIR}/${PROG}"
