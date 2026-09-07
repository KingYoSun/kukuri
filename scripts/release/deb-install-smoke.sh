#!/usr/bin/env bash
# System package changes are restricted to the disposable CI runner.
set -euo pipefail
test "${GITHUB_ACTIONS:-}" = true
deb=$(realpath "$1")
version=$2
test "$(dpkg-query -W -f='${db:Status-Status}' kukuri 2>/dev/null || true)" != installed
fixture=$(mktemp -d "${RUNNER_TEMP:?}/kukuri-deb-profile-XXXXXX")
mkdir -p "$fixture/app.kukuri.desktop"
printf 'preserved identity and DB sentinel\n' > "$fixture/app.kukuri.desktop/preserved"
before=$(sha256sum "$fixture/app.kukuri.desktop/preserved")
sudo apt-get install --yes "$deb"
test "$(dpkg-query -W -f='${Version}' kukuri)" = "$version"
test -x /usr/bin/kukuri-desktop-tauri
desktop-file-validate /usr/share/applications/kukuri.desktop
test -s /usr/share/doc/kukuri/copyright
test -s /usr/share/doc/kukuri/THIRD_PARTY_NOTICES.md
sudo apt-get install --yes --reinstall "$deb"
test "$before" = "$(sha256sum "$fixture/app.kukuri.desktop/preserved")"
sudo apt-get remove --yes kukuri
test ! -e /usr/bin/kukuri-desktop-tauri
test "$before" = "$(sha256sum "$fixture/app.kukuri.desktop/preserved")"
printf 'Deb install/reinstall/remove and data sentinel: passed\n'
