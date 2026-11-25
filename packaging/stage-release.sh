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
# Must match data/jmdict/README.md; a silent change of the vendored dictionary fails the build.
readonly JMDICT_SHA256="8bac55a4fd3ea2b103348ced2098a61165a73aaea45754e6535057c71d7b14fa"

log() { printf '[stage-release] %s\n' "$*" >&2; }

# Cargo that honours rust-toolchain.toml. Ubuntu's apt cargo (1.75) cannot read the
# 2024 edition, and rustup's ~/.cargo/bin is often missing from PATH in login shells.
find_cargo() {
    local channel candidate
    channel="$(sed -n 's/^channel = "\(.*\)"/\1/p' "$REPO_ROOT/service/rust-toolchain.toml")"
    for candidate in "${CARGO:-}" "${CARGO_HOME:-$HOME/.cargo}/bin/cargo" "$(command -v cargo || true)"; do
        [[ -n "$candidate" && -x "$candidate" ]] || continue
        if (cd "$REPO_ROOT/service" && "$candidate" --version 2>/dev/null) | grep -qF "cargo $channel"; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    log "Rust $channel não encontrado. Instale o rustup: curl https://sh.rustup.rs -sSf | sh"
    return 1
}

build_components() {
    log "building service (release)"
    local cargo
    cargo="$(find_cargo)"
    # rustup picks the toolchain from the working directory, so build from service/.
    (cd "$REPO_ROOT/service" && "$cargo" build --quiet --release --locked)
    log "packaging extension"
    node "$REPO_ROOT/extension/scripts/packageExtension.mjs" >/dev/null
}

# Builds the JMdict index from the vendored snapshot, so installs never download it.
build_dictionary_index() {
    local source="$REPO_ROOT/data/jmdict/JMdict_e.gz"
    echo "$JMDICT_SHA256  $source" | sha256sum --quiet -c - || { log "JMdict_e.gz hash mismatch"; exit 1; }
    log "indexing vendored JMdict"
    XDG_DATA_HOME="$STAGE/.index" "$STAGE/bin/gnome-furigana" build-jmdict "$source" > /dev/null
    install -m 0644 "$STAGE/.index/gnome-furigana/jmdict.sqlite" "$STAGE/data/jmdict.sqlite"
    install -m 0644 "$REPO_ROOT/data/jmdict/README.md" "$STAGE/data/JMdict-LICENSE.md"
    rm -rf "$STAGE/.index"
}

lay_out_stage() {
    rm -rf "$STAGE"
    install -d "$STAGE/bin" "$STAGE/extension" "$STAGE/schemas" "$STAGE/packaging" "$STAGE/data"
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
build_dictionary_index
archive_stage
