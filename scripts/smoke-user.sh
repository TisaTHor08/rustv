#!/bin/bash
# Second half of the smoke test, run as the unprivileged user `tester` (see smoke-linux.sh).
set -u
export HOME=/home/tester
cd "$HOME" || exit 1
export XDG_RUNTIME_DIR=/tmp/xdg-tester
mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
say() { echo "$*"; }

eval "$(dbus-launch --sh-syntax)"
export DBUS_SESSION_BUS_ADDRESS
pulseaudio --start --exit-idle-time=-1 --log-target=file:/tmp/pulse.log >/dev/null 2>&1
sleep 1
pactl load-module module-null-sink sink_name=rustv_dummy sink_properties=device.description=RusTV-Dummy >/dev/null 2>&1
pactl set-default-sink rustv_dummy >/dev/null 2>&1
say "pactl default sink: $(pactl get-default-sink 2>&1)"
eval "$(echo -n "" | gnome-keyring-daemon --unlock --components=secrets 2>/dev/null)"
export GNOME_KEYRING_CONTROL SSH_AUTH_SOCK 2>/dev/null
sleep 1

Xvfb :99 -screen 0 1920x1080x24 >/tmp/xvfb.log 2>&1 &
sleep 2
export DISPLAY=:99
openbox >/tmp/openbox.log 2>&1 &
sleep 1
# WebKitGTK sans GPU dans un conteneur
export WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 LIBGL_ALWAYS_SOFTWARE=1

# Appairage pre-enregistre pour le faux telephone (cle de 32 octets en base64)
KEY="$(head -c 32 /dev/urandom | base64)"
mkdir -p "$HOME/.config/RusTV"
cat > "$HOME/.config/RusTV/config.json" <<EOF
{"pc_id":"smoke0001","pc_name":"smoke","devices":[{"id":"aabbccdd","name":"Smoke phone","key":"$KEY","used":true}],"custom_apps":[]}
EOF

say "== start rustv --tv"
rustv --tv >/tmp/rustv.log 2>&1 &
PID=$!
sleep 20
if kill -0 "$PID" 2>/dev/null; then say "process alive after 20 s: YES (pid $PID)"; else say "process alive after 20 s: NO"; tail -30 /tmp/rustv.log; fi
say "port 47820: $(ss -ltn 2>/dev/null | grep -c ':47820 ') listener(s)"
say "config files: $(ls "$HOME/.config/RusTV" | tr '\n' ' ')"
say "windows: $(xdotool search --onlyvisible --name . 2>/dev/null | wc -l) visible"

mkdir -p /out
xwd -root -display :99 -silent 2>/dev/null | convert xwd:- /out/home.png 2>/dev/null && say "screenshot: /out/home.png" || say "screenshot: FAILED"

say "== phone handshake (fake_phone)"
timeout 20 "$HOME/tools/fake_phone" 127.0.0.1:47820 aabbccdd "$KEY" 2>&1 | head -12

say "== ignored live tests (real PulseAudio / keyring / uinput)"
for t in core shell; do
  [ -x "$HOME/tools/test-$t" ] && timeout 120 "$HOME/tools/test-$t" --ignored --test-threads=1 2>&1 | grep -E "^test |test result" | head -20
done

say "== app log (last lines)"
tail -15 /tmp/rustv.log
if kill -0 "$PID" 2>/dev/null; then say "still alive at the end: YES"; kill "$PID"; else say "still alive at the end: NO"; fi
