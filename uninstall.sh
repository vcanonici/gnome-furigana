#!/usr/bin/env bash
# gnome-furigana uninstaller: removes exactly what install.sh recorded in its manifest,
# disables the extension and, if the installer changed it, restores the session language.
# System files (/usr/share/locale, apt packages) are never touched.
#
# Exit codes: 0 ok · 1 failure · 2 invalid usage.
set -euo pipefail

readonly UUID="furigana@vcanonici.github.io"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
readonly DATA_HOME
readonly APP_DATA="$DATA_HOME/gnome-furigana"
readonly MANIFEST="$APP_DATA/install-manifest"
readonly STATE_FILE="$APP_DATA/install-state"
OPT_YES=0

log() { printf '\033[1;34m[furigana]\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31m[furigana]\033[0m %s\n' "$1" >&2; exit "${2:-1}"; }

confirm() {
    local answer=""
    ((OPT_YES)) && return 0
    [[ -r /dev/tty ]] || return 1
    read -r -p "$1 [s/N] " answer < /dev/tty || return 1
    [[ "$answer" =~ ^[sSyY]$ ]]
}

disable_extension() {
    local enabled
    enabled="$(gsettings get org.gnome.shell enabled-extensions)"
    enabled="$(printf '%s' "$enabled" | sed -e "s/, '$UUID'//; s/'$UUID', //; s/'$UUID'//")"
    gsettings set org.gnome.shell enabled-extensions "$enabled"
    gnome-extensions disable "$UUID" 2>/dev/null || true
}

stop_service() {
    systemctl --user stop gnome-furigana.service 2>/dev/null || true
}

# Only paths inside the user's own data/config directories are removed.
remove_recorded_paths() {
    local path
    [[ -f "$MANIFEST" ]] || { log "nenhum manifesto encontrado; nada a remover"; return; }
    while IFS= read -r path; do
        case "$path" in
            "$HOME"/.local/*|"$HOME"/.config/*|"$DATA_HOME"/*) rm -rf -- "$path" ;;
            *) log "ignorado (fora do diretório do usuário): $path" ;;
        esac
    done < "$MANIFEST"
    glib-compile-schemas "$DATA_HOME/glib-2.0/schemas" 2>/dev/null || true
    systemctl --user daemon-reload 2>/dev/null || true
}

restore_language() {
    local previous=""
    [[ -f "$STATE_FILE" ]] || return 0
    previous="$(sed -n 's/^previous_language=//p' "$STATE_FILE")"
    confirm "Restaurar o idioma anterior da sessão (${previous:-padrão do sistema})?" || return 0
    busctl call org.freedesktop.Accounts "/org/freedesktop/Accounts/User$(id -u)" \
        org.freedesktop.Accounts.User SetLanguage s "$previous" && log "idioma restaurado no próximo login"
}

main() {
    case "${1:-}" in
        --yes) OPT_YES=1 ;;
        -h|--help) printf 'Uso: uninstall.sh [--yes]\n'; exit 0 ;;
        "") ;;
        *) die "opção desconhecida: $1" 2 ;;
    esac
    disable_extension
    stop_service
    restore_language
    remove_recorded_paths
    rm -rf -- "$APP_DATA"
    log "gnome-furigana removido. Encerre a sessão para concluir."
}

main "$@"
