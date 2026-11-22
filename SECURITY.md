# Security policy

RusTV pairs a phone with a PC and can inject keyboard and mouse input, so security reports are taken seriously.

## Reporting a vulnerability

Please **do not open a public issue** for a vulnerability. Use GitHub private vulnerability reporting instead:
on the repository, open the **Security** tab and choose **Report a vulnerability**
(<https://github.com/TisaTHor08/rustv/security/advisories/new>).

Include what you found, how to reproduce it, the version you tested, and the impact you expect. You will get an
answer as soon as the maintainer can, and credit in the fix unless you prefer to stay anonymous. Please give us a
reasonable time to publish a fix before disclosing details.

## Supported versions

| Version | Supported |
| --- | --- |
| Latest release | Yes |
| Older releases | No, please update |

## Scope

In scope:

- **Pairing and the crypto protocol**: handshake, key handling, encrypted frames, anti-replay, revocation
  (see [docs/PROTOCOL.md](docs/PROTOCOL.md)).
- **The mod sandbox**: permission enforcement, sandboxed pages and widgets, site adapters, the catalog and its
  pinned-commit install path (see [docs/MODS.md](docs/MODS.md)).
- **The installer and update path**: update check, on-demand download of the mpv player (checksum), anything that
  downloads and runs code.
- Web app windows reaching RusTV commands or the file system.

Out of scope: vulnerabilities in third-party streaming sites, in WebView2 or Windows themselves, attacks that need
an already-compromised PC, and denial of service from a device on your own network that you paired yourself.
