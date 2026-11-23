#!/bin/bash
# Smoke test of the RusTV .deb, run INSIDE the smoke container (docker/linux/smoke.Dockerfile) as root.
# Mounts expected: /in (dist-linux with the .deb and tools/: fake_phone + test binaries), /out (screenshot, logs).
# It installs the package, starts a virtual X display, D-Bus, PulseAudio (null sink) and gnome-keyring, runs
# `rustv --tv` for 20 s and reports. Nothing here touches a real display, audio device or secret store.
set -u
mkdir -p /out && chmod 777 /out
REPORT=/out/smoke-report.txt
: > "$REPORT"
say() { echo "$*" | tee -a "$REPORT"; }

DEB="$(ls /in/*.deb | head -1)"
say "== install $DEB"
apt-get update -qq >/dev/null 2>&1
if apt-get install -y -qq "$DEB" >/out/apt-install.log 2>&1; then say "apt install: OK"; else say "apt install: FAILED"; tail -20 /out/apt-install.log | tee -a "$REPORT"; exit 1; fi
say "rustv --version: $(rustv --version 2>&1)"
say "missing shared libraries: $(ldd /usr/bin/rustv | grep -c 'not found')"
say "udev rule: $(ls /usr/lib/udev/rules.d/60-rustv-uinput.rules 2>&1)"
say "modules-load: $(cat /usr/lib/modules-load.d/rustv-uinput.conf 2>&1 | tail -1)"
say "desktop entry: $(grep -E '^(Exec|Icon|StartupWMClass|Categories)=' /usr/share/applications/RusTV.desktop 2>&1 | tr '\n' ' ')"

cp -r /in/tools /home/tester/tools && chown -R tester:tester /home/tester/tools
su tester -c "bash /scripts/smoke-user.sh" 2>&1 | tee -a "$REPORT"
