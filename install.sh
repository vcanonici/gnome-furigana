#!/usr/bin/env bash
# gnome-furigana installer — Ubuntu 24.04 · GNOME Shell 46 · Wayland.
#
#   curl -fsSL https://raw.githubusercontent.com/vcanonici/gnome-furigana/main/install.sh | bash
#   ./install.sh [options]            # from a release directory or a source checkout
#
# Per user: the linguistic service (Rust binary, D-Bus activated), the GNOME Shell
# extension, its GSettings schema and, when present, the Settings app with the
# "Furigana" panel. With sudo (skip with --no-system): annotated gettext catalogs
# `ja@furigana` for every GTK/gettext app, rebuilt after each apt run. Original
# catalogs are never modified. Changes take effect at next login.
#
# Exit codes: 0 ok · 1 failure · 2 invalid usage · 3 unsupported system.
set -euo pipefail

readonly REPOSITORY="vcanonici/gnome-furigana"
readonly UUID="furigana@vcanonici.github.io"
readonly SCHEMA_FILE="org.gnome.shell.extensions.furigana.gschema.xml"
readonly APT_PACKAGES=(language-pack-gnome-ja language-pack-ja fonts-noto-cjk gir1.2-adw-1)

DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}"
readonly DATA_HOME CONFIG_HOME
readonly APP_DATA="$DATA_HOME/gnome-furigana"
readonly MANIFEST="$APP_DATA/install-manifest"
readonly STATE_FILE="$APP_DATA/install-state"
readonly BACKUP_DIR="$APP_DATA/backup"
readonly RESTORE_LIST="$APP_DATA/restore-list"
readonly SYSTEM_MANIFEST="$APP_DATA/system-manifest"
readonly SYSTEM_LIB="/usr/local/lib/gnome-furigana"
readonly CATALOG_DIR="/usr/share/locale-langpack/ja@furigana"
readonly APT_HOOK="/etc/apt/apt.conf.d/99gnome-furigana"
readonly PAM_ENVIRONMENT="$HOME/.pam_environment"
readonly FURIGANA_LANGUAGE="ja@furigana"

OPT_YES=0
OPT_VERSION="latest"
OPT_SOURCE=""
OPT_LANGUAGE="ask"
OPT_JMDICT=1
OPT_SETTINGS_PANEL=1
OPT_APT=1
OPT_FORCE=0
OPT_SYSTEM=1
OPT_SETTINGS_FOLLOW_SESSION=0
WORK_DIR=""

usage() {
    cat <<'EOF'
Uso: install.sh [opções]

  --yes                 não perguntar; aceita os padrões (idioma: manter)
  --version X.Y.Z       versão do release a baixar (padrão: a mais recente)
  --source DIR          instalar de um diretório de release ou checkout local
  --language ja|keep    trocar a sessão para japonês ou manter (padrão: perguntar)
  --no-jmdict           não instalar o dicionário JMdict (traduções só do glossário)
  --no-settings-panel   não instalar o painel Furigana nas Configurações
  --no-apt              não instalar pacotes apt
  --no-system           sem sudo: não gerar catálogos ja@furigana (apps GTK sem furigana)
  --settings-follow-session  remover de um atalho próprio das Configurações o forçar
                        de idioma (env LANG=…), para elas seguirem a sessão
  --force               continuar mesmo fora de Ubuntu 24.04 / GNOME 46
  -h, --help            esta ajuda
EOF
}

log() { printf '\033[1;34m[furigana]\033[0m %s\n' "$*" >&2; }
warn() { printf '\033[1;33m[furigana]\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31m[furigana]\033[0m %s\n' "$1" >&2; exit "${2:-1}"; }

parse_args() {
    while (($#)); do
        case "$1" in
            --yes) OPT_YES=1 ;;
            --version) OPT_VERSION="${2:?--version precisa de um valor}"; shift ;;
            --source) OPT_SOURCE="${2:?--source precisa de um diretório}"; shift ;;
            --language) OPT_LANGUAGE="${2:?--language precisa de ja ou keep}"; shift ;;
            --no-jmdict) OPT_JMDICT=0 ;;
            --no-settings-panel) OPT_SETTINGS_PANEL=0 ;;
            --no-apt) OPT_APT=0 ;;
            --no-system) OPT_SYSTEM=0 ;;
            --settings-follow-session) OPT_SETTINGS_FOLLOW_SESSION=1 ;;
            --force) OPT_FORCE=1 ;;
            -h|--help) usage; exit 0 ;;
            *) usage >&2; die "opção desconhecida: $1" 2 ;;
        esac
        shift
    done
    [[ "$OPT_LANGUAGE" =~ ^(ask|ja|keep)$ ]] || die "--language deve ser ja ou keep" 2
    if ((OPT_YES)) && [[ "$OPT_LANGUAGE" == ask ]]; then OPT_LANGUAGE=keep; fi
}

# Interactive questions read from the terminal, so `curl | bash` still works.
confirm() {
    local answer=""
    ((OPT_YES)) && return 1
    [[ -r /dev/tty ]] || return 1
    read -r -p "$1 [s/N] " answer < /dev/tty || return 1
    [[ "$answer" =~ ^[sSyY]$ ]]
}

check_platform() {
    local os_id="" os_version="" shell_version=""
    # shellcheck source=/dev/null
    os_id="$(. /etc/os-release && printf '%s' "$ID")"
    # shellcheck source=/dev/null
    os_version="$(. /etc/os-release && printf '%s' "$VERSION_ID")"
    shell_version="$(gnome-shell --version 2>/dev/null | awk '{print $3}')"
    log "sistema: $os_id $os_version · GNOME Shell ${shell_version:-ausente} · sessão ${XDG_SESSION_TYPE:-?}"
    [[ "$(uname -m)" == x86_64 ]] || die "apenas x86_64 é suportado" 3
    if [[ "$os_id $os_version" != "ubuntu 24.04" || "${shell_version%%.*}" != 46 ]]; then
        ((OPT_FORCE)) || die "suportado apenas Ubuntu 24.04 + GNOME Shell 46 (use --force para tentar)" 3
        warn "sistema não suportado; continuando por causa de --force"
    fi
}

missing_apt_packages() {
    local package
    for package in "${APT_PACKAGES[@]}"; do
        dpkg-query -W -f='${Status}' "$package" 2>/dev/null | grep -q "install ok installed" || printf '%s\n' "$package"
    done
}

install_apt_packages() {
    local missing=()
    mapfile -t missing < <(missing_apt_packages)
    ((${#missing[@]})) || { log "pacotes apt: ok"; return; }
    if ((!OPT_APT)); then
        warn "pacotes ausentes (instale depois): ${missing[*]}"
        return
    fi
    log "instalando pacotes apt: ${missing[*]} (sudo)"
    sudo apt-get install -y --no-install-recommends "${missing[@]}"
}

# The newest release, prereleases included (`releases/latest` ignores prereleases).
latest_version() {
    curl -fsSL "https://api.github.com/repos/$REPOSITORY/releases?per_page=1" 2>/dev/null \
        | sed -n 's/.*"tag_name": *"v\{0,1\}\([^"]*\)".*/\1/p' | head -n1 || true
}

# Sets RELEASE_DIR to a directory laid out by packaging/stage-release.sh.
fetch_release() {
    if [[ -n "$OPT_SOURCE" ]]; then
        resolve_source_dir "$OPT_SOURCE"
        return
    fi
    local version="$OPT_VERSION" name url
    [[ "$version" != latest ]] || version="$(latest_version)"
    [[ -n "$version" ]] || die "não foi possível consultar os releases de $REPOSITORY (rede?); use --version X.Y.Z"
    name="gnome-furigana-${version}-x86_64"
    url="https://github.com/$REPOSITORY/releases/download/v$version/$name.tar.gz"
    log "baixando $name"
    curl -fsSL -o "$WORK_DIR/$name.tar.gz" "$url"
    curl -fsSL -o "$WORK_DIR/$name.tar.gz.sha256" "$url.sha256"
    (cd "$WORK_DIR" && sha256sum --quiet -c "$name.tar.gz.sha256") || die "checksum inválido para $name.tar.gz"
    tar -C "$WORK_DIR" -xzf "$WORK_DIR/$name.tar.gz"
    RELEASE_DIR="$WORK_DIR/$name"
}

# A source checkout is staged first; a release directory is used as is.
resolve_source_dir() {
    local source_dir
    source_dir="$(cd "$1" && pwd)"
    if [[ -f "$source_dir/VERSION" && -x "$source_dir/bin/gnome-furigana" ]]; then
        RELEASE_DIR="$source_dir"
    elif [[ -x "$source_dir/packaging/stage-release.sh" ]]; then
        log "compilando a partir do código-fonte"
        "$source_dir/packaging/stage-release.sh"
        RELEASE_DIR="$(find "$source_dir/dist" -mindepth 1 -maxdepth 1 -type d -name 'gnome-furigana-*' | sort | tail -n1)"
    else
        die "$source_dir não é um release nem um checkout do gnome-furigana" 2
    fi
}

record() { printf '%s\n' "$@" >> "$MANIFEST"; }

render_template() {
    sed "s|@BINDIR@|$APP_DATA/bin|g" "$1" > "$2"
    record "$2"
}

install_service() {
    log "serviço linguístico → $APP_DATA/bin"
    install -d "$APP_DATA/bin" "$CONFIG_HOME/systemd/user" "$DATA_HOME/dbus-1/services"
    install -m 0755 "$RELEASE_DIR/bin/gnome-furigana" "$APP_DATA/bin/gnome-furigana"
    record "$APP_DATA/bin/gnome-furigana"
    render_template "$RELEASE_DIR/packaging/gnome-furigana.service.in" "$CONFIG_HOME/systemd/user/gnome-furigana.service"
    render_template "$RELEASE_DIR/packaging/io.github.vcanonici.Furigana.service.in" \
        "$DATA_HOME/dbus-1/services/io.github.vcanonici.Furigana.service"
    systemctl --user daemon-reload 2>/dev/null || warn "systemd --user indisponível; ativação só via D-Bus"
    "$APP_DATA/bin/gnome-furigana" analyze 設定 > /dev/null || die "o serviço não executa neste sistema"
}

# The release ships the prebuilt index (vendored snapshot): no download at install time.
install_dictionary() {
    ((OPT_JMDICT)) || { log "JMdict: ignorado (--no-jmdict)"; return; }
    log "JMdict (EDRDG, CC BY-SA 4.0) → $APP_DATA/jmdict.sqlite"
    install -m 0644 "$RELEASE_DIR/data/jmdict.sqlite" "$APP_DATA/jmdict.sqlite"
    install -m 0644 "$RELEASE_DIR/data/JMdict-LICENSE.md" "$APP_DATA/JMdict-LICENSE.md"
    record "$APP_DATA/jmdict.sqlite" "$APP_DATA/JMdict-LICENSE.md"
}

install_schema() {
    local schema_dir="$DATA_HOME/glib-2.0/schemas"
    install -d "$schema_dir"
    install -m 0644 "$RELEASE_DIR/schemas/$SCHEMA_FILE" "$schema_dir/$SCHEMA_FILE"
    record "$schema_dir/$SCHEMA_FILE"
    glib-compile-schemas "$schema_dir"
}

install_extension() {
    local target="$DATA_HOME/gnome-shell/extensions/$UUID"
    log "extensão GNOME Shell → $target"
    rm -rf "$target"
    install -d "$target"
    unzip -q -o "$RELEASE_DIR/extension/$UUID.shell-extension.zip" -d "$target"
    record "$target"
    install_schema
    enable_extension
}

# Writes the setting directly: the running Shell only loads new extensions after
# the next login on Wayland, so nothing changes on screen now.
enable_extension() {
    local enabled
    enabled="$(gsettings get org.gnome.shell enabled-extensions)"
    if [[ "$enabled" != *"'$UUID'"* ]]; then
        if [[ "$enabled" == "@as []" || "$enabled" == "[]" ]]; then
            enabled="['$UUID']"
        else
            enabled="${enabled%]}, '$UUID']"
        fi
        gsettings set org.gnome.shell enabled-extensions "$enabled"
    fi
    # `disabled-extensions` wins over `enabled-extensions` in GNOME 46: a user who once
    # turned the extension off would otherwise stay without it after reinstalling.
    local disabled
    disabled="$(gsettings get org.gnome.shell disabled-extensions)"
    if [[ "$disabled" == *"'$UUID'"* ]]; then
        disabled="$(printf '%s' "$disabled" | sed -e "s/, '$UUID'//; s/'$UUID', //; s/'$UUID'//")"
        gsettings set org.gnome.shell disabled-extensions "$disabled"
    fi
    gsettings set org.gnome.shell disable-user-extensions false
}

install_settings_panel() {
    local target="$APP_DATA/settings"
    if ((!OPT_SETTINGS_PANEL)) || [[ ! -x "$RELEASE_DIR/settings/bin/gnome-control-center" ]]; then
        log "painel nas Configurações: não instalado (use as preferências da extensão)"
        return
    fi
    log "Configurações com painel Furigana → $target"
    rm -rf "$target"
    cp -a "$RELEASE_DIR/settings" "$target"
    record "$target"
    install -d "$DATA_HOME/applications"
    # The panel loader resolves gnome-<panel>-panel.desktop through XDG data dirs.
    install -m 0644 "$target/share/applications/gnome-furigana-panel.desktop" "$DATA_HOME/applications/"
    record "$DATA_HOME/applications/gnome-furigana-panel.desktop"
    override_launcher /usr/share/applications/org.gnome.Settings.desktop \
        "$DATA_HOME/applications/org.gnome.Settings.desktop" "$target/bin/gnome-control-center"
    override_launcher /usr/share/dbus-1/services/org.gnome.Settings.service \
        "$DATA_HOME/dbus-1/services/org.gnome.Settings.service" "$target/bin/gnome-control-center"
}

# Points a launcher (desktop entry or D-Bus service) at the fork. A user override that
# is not ours is backed up for uninstall.sh and used as the base, so wrappers such as
# `Exec=env LANG=… gnome-control-center` are kept and only the command is swapped.
override_launcher() {
    local base="$1" user_file="$2" binary="$3" saved
    saved="$BACKUP_DIR/$(basename "$user_file")"
    if [[ -f "$user_file" ]] && ! grep -q "gnome-furigana" "$user_file"; then
        backup_user_file "$user_file"
        log "seu $(basename "$user_file") foi preservado (backup em $BACKUP_DIR)"
    fi
    [[ -f "$saved" ]] && base="$saved"
    sed -E "s#^(Exec=(.*[ ])?)(/usr/bin/)?gnome-control-center( |\$)#\1$binary\4#" "$base" > "$user_file"
    if ((OPT_SETTINGS_FOLLOW_SESSION)); then
        # Drops `env VAR=value …` wrappers so Settings follows the session language.
        sed -i -E 's#^Exec=env ([A-Za-z_]+=[^ ]+ )+#Exec=#' "$user_file"
    fi
    record "$user_file"
}

# Saves a user file once, before our first change, for uninstall.sh. A file that did
# not exist gets an `.absent` marker, so a reinstall never mistakes our version for
# the user's.
backup_user_file() {
    local path="$1" saved
    saved="$BACKUP_DIR/$(basename "$path")"
    [[ -e "$saved" || -e "$saved.absent" ]] && return
    install -d "$BACKUP_DIR"
    if [[ -f "$path" ]]; then
        cp -p "$path" "$saved"
    else
        printf '%s\n' "$path" > "$saved.absent"
    fi
    printf '%s\n' "$path" >> "$RESTORE_LIST"
}

# Puts ja@furigana first in the session's LANGUAGE (read by PAM at login). glibc then
# takes annotated messages from it and everything else from plain `ja`.
configure_furigana_language() {
    local current_list
    if [[ "$(current_language)" != ja* ]]; then
        log "sessão não está em japonês: catálogos com furigana não ativados"
        return
    fi
    current_list="$(sed -n 's/^LANGUAGE[[:space:]]\+DEFAULT=//p' "$PAM_ENVIRONMENT" 2>/dev/null | head -n1)"
    if [[ ":$current_list:" == *":$FURIGANA_LANGUAGE:"* ]]; then
        log "sessão: $FURIGANA_LANGUAGE já ativo"
        return
    fi
    backup_user_file "$PAM_ENVIRONMENT"
    local new_list="$FURIGANA_LANGUAGE:${current_list:-ja}"
    if grep -q '^LANGUAGE[[:space:]]' "$PAM_ENVIRONMENT" 2>/dev/null; then
        sed -i -E "s#^LANGUAGE[[:space:]]+DEFAULT=.*#LANGUAGE\tDEFAULT=$new_list#" "$PAM_ENVIRONMENT"
    else
        printf 'LANGUAGE\tDEFAULT=%s\n' "$new_list" >> "$PAM_ENVIRONMENT"
    fi
    log "sessão: LANGUAGE=$new_list a partir do próximo login"
}

# Root-owned copy of the binary (root never runs the user-writable one), catalogs and
# an apt hook that refreshes them after package changes.
install_system_catalogs() {
    if ((!OPT_SYSTEM)); then
        log "catálogos ja@furigana: ignorados (--no-system)"
        return
    fi
    log "catálogos com furigana para os apps GTK → $CATALOG_DIR (pede a senha de sudo)"
    sudo install -d -o root -g root -m 0755 "$SYSTEM_LIB"
    sudo install -o root -g root -m 0755 "$RELEASE_DIR/bin/gnome-furigana" "$SYSTEM_LIB/gnome-furigana"
    printf 'DPkg::Post-Invoke { "if [ -x %s/gnome-furigana ]; then %s/gnome-furigana build-catalogs --quiet || true; fi"; };\n' \
        "$SYSTEM_LIB" "$SYSTEM_LIB" | sudo tee "$APT_HOOK" > /dev/null
    printf '%s\n' "$SYSTEM_LIB" "$CATALOG_DIR" "$APT_HOOK" > "$SYSTEM_MANIFEST"
    sudo "$SYSTEM_LIB/gnome-furigana" build-catalogs --quiet || warn "alguns catálogos falharam; os demais foram gerados"
}

current_language() {
    busctl get-property org.freedesktop.Accounts "/org/freedesktop/Accounts/User$(id -u)" \
        org.freedesktop.Accounts.User Language 2>/dev/null | awk -F'"' '{print $2}'
}

configure_language() {
    local previous
    previous="$(current_language)"
    if [[ "$previous" == ja_JP* ]]; then log "idioma da sessão: japonês"; return; fi
    case "$OPT_LANGUAGE" in
        keep) log "idioma da sessão mantido (${previous:-padrão})"; return ;;
        ask) confirm "Mudar o idioma da sua sessão GNOME para japonês (efeito no próximo login)?" || return 0 ;;
    esac
    printf 'previous_language=%s\n' "$previous" > "$STATE_FILE"
    if busctl call org.freedesktop.Accounts "/org/freedesktop/Accounts/User$(id -u)" \
        org.freedesktop.Accounts.User SetLanguage s ja_JP.UTF-8; then
        log "idioma: japonês a partir do próximo login"
    else
        warn "não foi possível trocar o idioma; use Configurações → Sistema → Região e idioma"
    fi
}

main() {
    parse_args "$@"
    WORK_DIR="$(mktemp -d)"
    trap 'rm -rf "$WORK_DIR"' EXIT
    check_platform
    install_apt_packages
    fetch_release
    install -d "$APP_DATA"
    : > "$MANIFEST"
    install_service
    install_dictionary
    install_extension
    install_settings_panel
    configure_language
    configure_furigana_language
    install_system_catalogs
    cp "$RELEASE_DIR/VERSION" "$APP_DATA/VERSION"
    log "pronto: versão $(cat "$APP_DATA/VERSION"). Encerre a sessão e entre de novo para ativar."
}

main "$@"
