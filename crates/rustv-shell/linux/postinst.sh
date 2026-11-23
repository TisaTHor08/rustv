#!/bin/sh
# RusTV : active tout de suite la regle udev et le module uinput (sans redemarrer). Jamais bloquant.
set -e
modprobe uinput 2>/dev/null || true
if command -v udevadm >/dev/null 2>&1; then
  udevadm control --reload-rules 2>/dev/null || true
  udevadm trigger --subsystem-match=misc --sysname-match=uinput 2>/dev/null || true
fi
exit 0
