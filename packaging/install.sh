#!/usr/bin/env bash
# install.sh — install quotabus's central probe on this host or on a named host (CHORE-008; DESIGN §3, §10 Q2).
#
#   packaging/install.sh (--where local | --host <name>) [--user <u>] [--home <dir>] [--os macos|linux]
#                        [--launcher "<cmd and args>"] [--quotabus <path>] [--probe-config <path>]
#                        [--interval <seconds>] [--render-to <dir>]
#
# With neither --where nor --host it asks on stdin ("local" or a host name).
#   local   this host, as the current user: --user/--home default to $USER/$HOME, --os to this host's OS.
#   --host  a named host over SSH: --user and --home are required (nothing connects to discover them); --os
#           defaults to macos.
# Units: macOS -> a launchd agent <home>/Library/LaunchAgents/com.congruentsys.quotabus-probe.plist, logging to
#        <home>/Library/Logs/quotabus/; Linux -> a systemd user unit + timer in <home>/.config/systemd/user/.
# --launcher is the command that injects the keys into the probe's environment, split on whitespace and placed
#   before the quotabus binary, e.g. "/opt/homebrew/bin/doppler run --project P --config C --" or
#   "secretspec run --". Default: none. No secret is ever written into a unit or put on argv: keys arrive only
#   through the launcher's environment.
# --render-to <dir> is a dry run: it writes the unit(s) into <dir> and prints the plan; it connects to nothing,
#   loads nothing and writes nothing under the target home.
# Portable bash (3.2+).
set -eu

LABEL="com.congruentsys.quotabus-probe"
PLIST_NAME="$LABEL.plist"
SERVICE_NAME="quotabus-probe.service"
TIMER_NAME="quotabus-probe.timer"

die() { echo "install.sh: $*" >&2; exit 2; }

where="" host="" user="" home="" os="" render_to=""
launcher="" qb_bin="/usr/local/bin/quotabus" probe_config="/usr/local/etc/quotabus/quotabus.toml" interval="900"

need() { [ $# -ge 2 ] && [ -n "$2" ] || die "$1 needs a value"; }
while [ $# -gt 0 ]; do
    case "$1" in
        --where) need "$@"; where="$2"; shift 2 ;;
        --host) need "$@"; host="$2"; shift 2 ;;
        --user) need "$@"; user="$2"; shift 2 ;;
        --home) need "$@"; home="$2"; shift 2 ;;
        --os) need "$@"; os="$2"; shift 2 ;;
        --render-to) need "$@"; render_to="$2"; shift 2 ;;
        --launcher) [ $# -ge 2 ] || die "--launcher needs a value"; launcher="$2"; shift 2 ;;
        --quotabus) need "$@"; qb_bin="$2"; shift 2 ;;
        --probe-config) need "$@"; probe_config="$2"; shift 2 ;;
        --interval) need "$@"; interval="$2"; shift 2 ;;
        -h|--help) sed -n '2,22p' "$0"; exit 0 ;;
        *) die "unknown argument: $1" ;;
    esac
done

# ---- validate everything that needs no answer first
case "$where" in
    ""|local) ;;
    *) die "unknown --where value: $where (use --where local or --host <name>)" ;;
esac
[ -z "$where" ] || [ -z "$host" ] || die "give --where local or --host <name>, not both"
case "$os" in
    ""|macos|linux) ;;
    *) die "unknown --os value: $os (macos or linux)" ;;
esac
case "$interval" in
    ""|*[!0-9]*) die "--interval must be a whole number of seconds: $interval" ;;
esac
[ "$interval" -gt 0 ] || die "--interval must be positive: $interval"

# ---- the target: a flag, else ask
if [ -z "$where" ] && [ -z "$host" ]; then
    [ -t 0 ] && printf 'Where does the central probe run? "local" (this host) or a host name: ' >&2
    answer=""
    IFS= read -r answer || true
    answer=$(printf '%s' "$answer" | tr -d '[:space:]')
    [ -n "$answer" ] || die "no target: answer \"local\" or a host name, or pass --where local / --host <name>"
    if [ "$answer" = "local" ]; then where="local"; else host="$answer"; fi
fi

if [ -n "$host" ]; then
    [ -n "$user" ] || die "--host $host needs --user (the account on $host that runs the probe)"
    [ -n "$home" ] || die "--host $host needs --home (that account's home directory on $host)"
    [ -n "$os" ] || os="macos"
else
    [ -n "$user" ] || user="${USER:-$(id -un)}"
    [ -n "$home" ] || home="${HOME:?HOME is not set}"
    if [ -z "$os" ]; then
        case "$(uname -s)" in
            Darwin) os="macos" ;;
            Linux) os="linux" ;;
            *) die "unknown --os value: $(uname -s) (pass --os macos or --os linux)" ;;
        esac
    fi
fi
case "$home" in /*) ;; *) die "--home must be absolute: $home" ;; esac
home="${home%/}"

# ---- rendering
xml_escape() { printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'; }

# argv = launcher words + quotabus probe --config <file>; one word per line
program_args() {
    local w
    # word-split the launcher on whitespace, without globbing
    set -f
    for w in $launcher; do printf '%s\n' "$w"; done
    set +f
    printf '%s\n' "$qb_bin" probe --config "$probe_config"
}

render_plist() {
    local w
    cat <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>$LABEL</string>
    <key>ProgramArguments</key>
    <array>
EOF
    program_args | while IFS= read -r w; do
        printf '        <string>%s</string>\n' "$(xml_escape "$w")"
    done
    cat <<EOF
    </array>
    <key>StartInterval</key>
    <integer>$interval</integer>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$(xml_escape "$home/Library/Logs/quotabus/probe.out.log")</string>
    <key>StandardErrorPath</key>
    <string>$(xml_escape "$home/Library/Logs/quotabus/probe.err.log")</string>
</dict>
</plist>
EOF
}

systemd_quote() {
    case "$1" in
        *[[:space:]\"\\]*) printf '"%s"' "$(printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g')" ;;
        *) printf '%s' "$1" ;;
    esac
}

render_service() {
    local w line=""
    while IFS= read -r w; do
        line="$line${line:+ }$(systemd_quote "$w")"
    done <<EOF
$(program_args)
EOF
    cat <<EOF
[Unit]
Description=quotabus central probe (one cycle; run by $TIMER_NAME)
After=network-online.target

[Service]
Type=oneshot
ExecStart=$line
EOF
}

render_timer() {
    cat <<EOF
[Unit]
Description=Run the quotabus central probe every $interval s

[Timer]
OnBootSec=60
OnUnitActiveSec=$interval
Unit=$SERVICE_NAME

[Install]
WantedBy=timers.target
EOF
}

if [ "$os" = "macos" ]; then
    dest_dir="$home/Library/LaunchAgents"
    log_dir="$home/Library/Logs/quotabus"
    units="$PLIST_NAME"
else
    dest_dir="$home/.config/systemd/user"
    log_dir=""
    units="$SERVICE_NAME $TIMER_NAME"
fi

render_into() {
    local dir="$1"
    if [ "$os" = "macos" ]; then
        render_plist > "$dir/$PLIST_NAME"
    else
        render_service > "$dir/$SERVICE_NAME"
        render_timer > "$dir/$TIMER_NAME"
    fi
}

print_plan() {
    local u
    if [ -n "$host" ]; then echo "target: host $host (over ssh, as $user)"; else echo "target: local (this host, as $user)"; fi
    echo "os: $os"
    echo "home: $home"
    for u in $units; do echo "unit: $u -> $dest_dir/$u"; done
    [ -z "$log_dir" ] || echo "logs: $log_dir/"
    if [ -n "$launcher" ]; then echo "launcher: $launcher"; else echo "launcher: (none: the unit's environment must carry the keys)"; fi
    echo "interval: ${interval}s"
}

# ---- dry run
if [ -n "$render_to" ]; then
    [ -d "$render_to" ] || mkdir -p "$render_to"
    render_into "$render_to"
    echo "dry run (--render-to $render_to): nothing installed"
    print_plan
    exit 0
fi

# ---- real install
stage=$(mktemp -d "${TMPDIR:-/tmp}/quotabus-install.XXXXXX")
trap 'rm -rf "$stage"' EXIT
render_into "$stage"
print_plan

if [ "$os" = "macos" ]; then
    load_cmd="launchctl unload '$dest_dir/$PLIST_NAME' 2>/dev/null || true; launchctl load -w '$dest_dir/$PLIST_NAME'"
    mkdir_cmd="mkdir -p '$dest_dir' '$log_dir'"
else
    load_cmd="systemctl --user daemon-reload && systemctl --user enable --now $TIMER_NAME"
    mkdir_cmd="mkdir -p '$dest_dir'"
fi

if [ -z "$host" ]; then
    sh -c "$mkdir_cmd"
    for u in $units; do cp "$stage/$u" "$dest_dir/$u"; done
    sh -c "$load_cmd"
else
    target="$user@$host"
    ssh "$target" "$mkdir_cmd"
    for u in $units; do scp -q "$stage/$u" "$target:$dest_dir/$u"; done
    ssh "$target" "$load_cmd"
fi
echo "installed: $units on ${host:-this host}"
