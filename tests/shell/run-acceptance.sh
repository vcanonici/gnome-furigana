#!/usr/bin/env bash
# End-to-end acceptance test of gnome-furigana inside a throwaway headless GNOME Shell 46.
# Run INSIDE a test VM (never on a real desktop session) after install.sh:
#
#   tests/shell/run-acceptance.sh [output-dir]
#
# Starts `gnome-shell --headless` in a private D-Bus session with the Japanese locale,
# drives the Quick Settings menu and a virtual pointer through org.gnome.Shell.Eval,
# and asserts the acceptance criteria (CA-F01, CA-H01..H05, CA-F07). Screenshots are
# written to the output directory. Exit code 0 when every check passes.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly HERE
readonly OUT="${1:-$PWD/acceptance-output}"
readonly UUID="furigana@vcanonici.github.io"
readonly HELPER="unsafe-mode@furigana-tests"
readonly EXTENSIONS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions"
# Test sessions use their own dconf database: a second dconf-service writing the user's
# real database clobbers settings and destabilises Shell startup.
readonly TEST_DCONF_DB="furigana_acceptance"  # becomes a D-Bus path element: no hyphens
FAILURES=0

log() { printf '[acceptance] %s\n' "$*" >&2; }

if [[ "${INSIDE_TEST_SESSION:-}" != 1 ]]; then
    mkdir -p "$OUT"
    rm -rf "${EXTENSIONS_DIR:?}/$HELPER"
    cp -r "$HERE/$HELPER" "$EXTENSIONS_DIR/"
    rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/dconf/$TEST_DCONF_DB"
    printf 'user-db:%s\n' "$TEST_DCONF_DB" > "$OUT/dconf-profile"
    exec env INSIDE_TEST_SESSION=1 LANG=ja_JP.UTF-8 LANGUAGE=ja LC_ALL=ja_JP.UTF-8 \
        DCONF_PROFILE="$OUT/dconf-profile" dbus-run-session -- "$0" "$OUT"
fi

shell_eval() {
    local reply
    reply="$(gdbus call --session --timeout 5 --dest org.gnome.Shell --object-path /org/gnome/Shell \
        --method org.gnome.Shell.Eval "$1" 2>/dev/null)" || { printf 'null'; return; }
    # Reply looks like (true, '"json"'): GVariant booleans are not Python literals, so
    # split off the flag and parse only the quoted payload.
    python3 -c '
import ast, sys
flag, _, rest = sys.argv[1].strip()[1:-1].partition(", ")
print(ast.literal_eval(rest) if flag == "true" else "null")' "$reply"
}

json_field() {
    python3 -c 'import json,sys; v=json.loads(sys.argv[1]); [v := v[k] if not isinstance(v, list) else v[int(k)] for k in sys.argv[2:]]; print(json.dumps(v, ensure_ascii=False))' "$@"
}

check() {
    local name="$1" actual="$2" expected="$3"
    if [[ "$actual" == "$expected" ]]; then
        log "PASS $name"
    else
        log "FAIL $name: expected $expected, got $actual"
        FAILURES=$((FAILURES + 1))
    fi
}

screenshot() {
    gdbus call --session --timeout 10 --dest org.gnome.Shell.Screenshot --object-path /org/gnome/Shell/Screenshot \
        --method org.gnome.Shell.Screenshot.Screenshot false false "$OUT/$1.png" > /dev/null || log "screenshot $1 failed"
}

start_shell() {
    # A Shell killed during startup leaves this crash guard behind, and the next Shell
    # then starts with every extension disabled. Safe to clear in a throwaway session.
    rm -f "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/gnome-shell-disable-extensions"
    # Sockets left by a killed Xwayland make the next one hang during Shell startup.
    for _ in $(seq 1 10); do pgrep -x 'Xwayland|gnome-shell' > /dev/null || break; sleep 0.5; done
    if pgrep -x 'Xwayland|gnome-shell' > /dev/null; then
        log "another compositor is running; refusing to start a test Shell"
        exit 1
    fi
    # Leftovers of a killed compositor (sockets, locks) make the next startup hang.
    rm -f /tmp/.X11-unix/X* /tmp/.X*-lock "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"/wayland-*
    gsettings set org.gnome.shell enabled-extensions "['$HELPER', '$UUID']"
    # --no-x11: Xwayland intermittently fails to attach in headless VMs and stalls startup.
    gnome-shell --headless --wayland --no-x11 --virtual-monitor 1280x800 > "$OUT/gnome-shell.log" 2>&1 &
    SHELL_PID=$!
    # Calling org.gnome.Shell.Eval while the Shell 46 is still starting blocks its
    # startup, so wait for the log line before the first D-Bus call.
    for _ in $(seq 1 60); do
        grep -q "GNOME Shell started" "$OUT/gnome-shell.log" && break
        sleep 1
    done
    local ready=""
    for attempt in $(seq 1 20); do
        ready="$(shell_eval '!Main.layoutManager._startingUp && global.context.unsafe_mode')"
        log "waiting for Shell ($attempt): $ready"
        [[ "$ready" == true ]] && return
        sleep 1
    done
    log "Shell did not start; see $OUT/gnome-shell.log"
    exit 1
}

# shellcheck disable=SC2317  # invoked through the EXIT trap
stop_shell() {
    [[ -n "$SHELL_PID" ]] || return 0
    kill "$SHELL_PID" 2>/dev/null || true
    # A Shell stalled in startup ignores SIGTERM; do not leave it behind.
    for _ in $(seq 1 10); do kill -0 "$SHELL_PID" 2>/dev/null || break; sleep 0.5; done
    kill -9 "$SHELL_PID" 2>/dev/null || true
    pkill -9 -x Xwayland 2>/dev/null || true
    wait "$SHELL_PID" 2>/dev/null || true
}

run_scenarios() {
    local state point
    check "probe" "$(shell_eval "$(cat "$HERE/acceptanceProbe.js")")" '"probe-installed"'
    log "extension: $(shell_eval "(({state, error}) => ({state, error}))(Main.extensionManager.lookup('$UUID'))") enabled=$(shell_eval 'Main.extensionManager._enabledExtensions')"
    check "extension state ACTIVE" "$(shell_eval "Main.extensionManager.lookup('$UUID').state")" 1
    shell_eval 'furiganaProbe.openQuickSettings()' > /dev/null
    sleep 2
    state="$(shell_eval 'furiganaProbe.state()')"
    printf '%s\n' "$state" > "$OUT/state-open.json"
    check "CA-F01 ruby visible over Quick Settings" "$(json_field "$state" ruby | python3 -c 'import json,sys; print(len(json.load(sys.stdin)) > 0)')" True
    screenshot quick-settings-ruby
    log "top bar: $(json_field "$state" panelLabels)"
    check "top bar clock annotated (inline, panel has no room above)" \
        "$(json_field "$state" panelLabels | python3 -c 'import json,sys; print(any("(" in t for t in json.load(sys.stdin)))')" True

    point="$(shell_eval "furiganaProbe.hover('設定')")"
    [[ "$point" == null ]] && point="$(shell_eval "furiganaProbe.hover('電源')")"
    log "hovering $point"
    sleep 1.5
    shell_eval 'furiganaProbe.nudge(1, 0)' > /dev/null
    sleep 0.3
    check "CA-H01 no tooltip before 2000 ms" "$(json_field "$(shell_eval 'furiganaProbe.state()')" tooltipVisible)" false
    sleep 0.8
    state="$(shell_eval 'furiganaProbe.state()')"
    check "CA-H02 tooltip after 2000 ms despite small move" "$(json_field "$state" tooltipVisible)" true
    log "tooltip: $(json_field "$state" tooltipText)"
    screenshot hover-tooltip

    shell_eval 'furiganaProbe.leave()' > /dev/null
    sleep 0.3
    check "CA-H04 leaving hides the tooltip" "$(json_field "$(shell_eval 'furiganaProbe.state()')" tooltipVisible)" false

    shell_eval 'furiganaProbe.closeQuickSettings()' > /dev/null
    sleep 0.5
    check "menu closed hides ruby" "$(json_field "$(shell_eval 'furiganaProbe.state()')" ruby)" '[]'

    shell_eval "Main.extensionManager.disableExtension('$UUID')" > /dev/null
    sleep 1
    state="$(shell_eval 'furiganaProbe.state()')"
    check "CA-F07 disable removes all ruby actors" "$(json_field "$state" rubyTotal)" 0
    shell_eval 'furiganaProbe.openQuickSettings()' > /dev/null
    sleep 1
    printf '%s\n' "$(shell_eval 'furiganaProbe.state()')" > "$OUT/state-disabled.json"
    screenshot quick-settings-disabled
}

SHELL_PID=""
trap stop_shell EXIT
trap "exit 130" INT TERM
start_shell
run_scenarios
grep -iE "furigana|JS ERROR" "$OUT/gnome-shell.log" > "$OUT/shell-errors.log" || true
log "screenshots and logs in $OUT; failures: $FAILURES"
exit $((FAILURES > 0))
