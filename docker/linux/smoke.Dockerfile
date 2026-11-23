# Image de test de fumee RusTV sous Linux : affichage virtuel (Xvfb), D-Bus, PulseAudio (sortie factice),
# trousseau (gnome-keyring), mpv, outils de capture. Le paquet .deb est installe au lancement (voir scripts/smoke-linux.sh).
FROM ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        xvfb openbox dbus dbus-x11 pulseaudio pulseaudio-utils mpv xdotool x11-apps imagemagick \
        gnome-keyring libsecret-tools iproute2 procps ca-certificates fonts-dejavu-core xdg-utils \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -s /bin/bash tester
