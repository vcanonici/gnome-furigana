#!/usr/bin/env bash
# Builds every component and lays out the release directory consumed by install.sh:
#   dist/gnome-furigana-<version>-x86_64/{bin,extension,schemas,packaging,VERSION}
#   dist/gnome-furigana-<version>-x86_64.tar.gz + .sha256
# Optional: SETTINGS_FORK_DIR=<dir with bin/gnome-control-center> adds settings/.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$REPO_ROOT/service/Cargo.toml" | head -n1)"
readonly REPO_ROOT VERSION
readonly NAME="gnome-furigana-${VERSION}-x86_64"
readonly STAGE="$REPO_ROOT/dist/$NAME"
readonly UUID="furigana@vcanonici.github.io"

log() { printf '[stage-release] %s\n' "$*" >&2; }

build_components() {
    log "building service (release)"
    cargo build --quiet --release --locked --manifest-path "$REPO_ROOT/service/Cargo.toml"
    log "packaging extension"
    node "$REPO_ROOT/extension/scripts/packageExtension.mjs" >/dev/null
}

lay_out_stage() {
    rm -rf "$STAGE"
    install -d "$STAGE/bin" "$STAGE/extension" "$STAGE/schemas" "$STAGE/packaging"
    install -m 0755 "$REPO_ROOT/service/target/release/gnome-furigana" "$STAGE/bin/"
    install -m 0644 "$REPO_ROOT/build/$UUID.shell-extension.zip" "$STAGE/extension/"
    install -m 0644 "$REPO_ROOT/extension/schemas/"*.gschema.xml "$STAGE/schemas/"
    install -m 0644 "$REPO_ROOT/packaging/"*.service.in "$STAGE/packaging/"
    install -m 0755 "$REPO_ROOT/install.sh" "$REPO_ROOT/uninstall.sh" "$STAGE/"
    if [[ -n "${SETTINGS_FORK_DIR:-}" ]]; then
        log "adding Settings fork from $SETTINGS_FORK_DIR"
        cp -a "$SETTINGS_FORK_DIR" "$STAGE/settings"
    fi
    printf '%s\n' "$VERSION" > "$STAGE/VERSION"
}

archive_stage() {
    tar -C "$REPO_ROOT/dist" -czf "$REPO_ROOT/dist/$NAME.tar.gz" "$NAME"
    (cd "$REPO_ROOT/dist" && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
    log "ready: dist/$NAME.tar.gz"
}

build_components
lay_out_stage
archive_stage
