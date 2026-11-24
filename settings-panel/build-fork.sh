#!/usr/bin/env bash
# Builds GNOME Settings (gnome-control-center) from Ubuntu 24.04's own source package,
# adds the "Furigana" panel and writes a relocatable install tree to OUTPUT_DIR:
#
#   OUTPUT_DIR/bin/gnome-control-center
#   OUTPUT_DIR/share/applications/gnome-furigana-panel.desktop
#   OUTPUT_DIR/VERSION                      (Ubuntu package version it was built from)
#
# Only the binary is taken: it is configured with prefix /usr and Ubuntu's flags, so it
# reuses the identical data files of the installed package. Runs as root in an
# ubuntu:24.04 container (CI) or with sudo in a disposable VM.
#
# Usage: settings-panel/build-fork.sh OUTPUT_DIR
set -euo pipefail

PANEL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/furigana"
readonly PANEL_DIR
readonly OUTPUT_DIR="${1:?usage: build-fork.sh OUTPUT_DIR}"
readonly WORK_DIR="${WORK_DIR:-$(mktemp -d)}"
SUDO=""
[[ "$(id -u)" == 0 ]] || SUDO="sudo"

log() { printf '[build-fork] %s\n' "$*" >&2; }

# Replaces exactly one occurrence of a literal line; fails loudly if upstream changed.
insert_after() {
    local file="$1" anchor="$2" addition="$3"
    [[ "$(grep -cxF -- "$anchor" "$file")" == 1 ]] || { log "anchor not found once in $file: $anchor"; exit 1; }
    awk -v anchor="$anchor" -v addition="$addition" '{print} $0 == anchor {print addition}' "$file" > "$file.new"
    mv "$file.new" "$file"
}

fetch_source() {
    log "fetching Ubuntu source of gnome-control-center"
    export DEBIAN_FRONTEND=noninteractive
    # Fresh machines run unattended-upgrades at boot; wait for its dpkg lock.
    local apt=(apt-get -o DPkg::Lock::Timeout=900)
    $SUDO sed -i 's/^Types: deb$/Types: deb deb-src/' /etc/apt/sources.list.d/ubuntu.sources
    $SUDO "${apt[@]}" update -qq
    $SUDO "${apt[@]}" install -y -qq dpkg-dev libjson-glib-dev > /dev/null
    $SUDO "${apt[@]}" build-dep -y -qq gnome-control-center > /dev/null
    (cd "$WORK_DIR" && apt-get source -qq gnome-control-center > /dev/null)
    SOURCE_DIR="$(find "$WORK_DIR" -mindepth 1 -maxdepth 1 -type d -name 'gnome-control-center-*' | head -n1)"
    PACKAGE_VERSION="$(cd "$SOURCE_DIR" && dpkg-parsechangelog -S Version)"
}

add_panel() {
    log "adding the Furigana panel to $PACKAGE_VERSION"
    cp -r "$PANEL_DIR" "$SOURCE_DIR/panels/furigana"
    insert_after "$SOURCE_DIR/panels/meson.build" "  'universal-access'," "  'furigana',"
    insert_after "$SOURCE_DIR/shell/cc-panel-loader.c" "extern GType cc_ua_panel_get_type (void);" \
        "extern GType cc_furigana_panel_get_type (void);"
    insert_after "$SOURCE_DIR/shell/cc-panel-loader.c" \
        '  PANEL_TYPE("universal-access", cc_ua_panel_get_type,                   NULL),' \
        '  PANEL_TYPE("furigana",         cc_furigana_panel_get_type,             NULL),'
    insert_after "$SOURCE_DIR/shell/cc-panel-list.c" '  "universal-access",' '  "furigana",'
}

# Same options as Ubuntu's debian/rules (override_dh_auto_configure), docs/tests off.
build_binary() {
    log "configuring and building"
    # Meson shlex-splits CFLAGS, so the escaped quotes reach the compiler intact.
    CFLAGS='-O2 -DSSHD_SERVICE=\"ssh.service\" -DSSHD_SOCKET=\"ssh.socket\"' \
        meson setup "$WORK_DIR/build" "$SOURCE_DIR" --prefix=/usr --buildtype=plain \
        -Db_lto=false -Dlocation-services=enabled -Dprivileged_group=sudo -Dwhoopsie=true \
        -Ddocumentation=false -Dtests=false -Dsnap=true -Dmalcontent=true \
        -Ddistributor_logo=/usr/share/pixmaps/ubuntu-logo-text.png \
        -Ddark_mode_distributor_logo=/usr/share/pixmaps/ubuntu-logo-text-dark.png > "$WORK_DIR/meson.log"
    ninja -C "$WORK_DIR/build" shell/gnome-control-center panels/furigana/gnome-furigana-panel.desktop
}

install_tree() {
    rm -rf "$OUTPUT_DIR"
    install -d "$OUTPUT_DIR/bin" "$OUTPUT_DIR/share/applications"
    install -m 0755 "$WORK_DIR/build/shell/gnome-control-center" "$OUTPUT_DIR/bin/"
    install -m 0644 "$WORK_DIR/build/panels/furigana/gnome-furigana-panel.desktop" "$OUTPUT_DIR/share/applications/"
    printf '%s\n' "$PACKAGE_VERSION" > "$OUTPUT_DIR/VERSION"
    log "done: $OUTPUT_DIR ($PACKAGE_VERSION)"
}

fetch_source
add_panel
build_binary
install_tree
