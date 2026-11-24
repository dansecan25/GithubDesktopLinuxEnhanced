#!/usr/bin/env bash
# Per-user install, no sudo needed. Run from the extracted release folder.
#   ./install.sh               install or update
#   ./install.sh --uninstall   remove (settings in ~/.config/github-desktop-enhanced are kept)
set -euo pipefail

NAME="github-desktop-enhanced"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}"
APP_DIR="${DATA_DIR}/applications"
ICON_DIR="${DATA_DIR}/icons/hicolor/scalable/apps"

refresh_caches() {
  command -v update-desktop-database >/dev/null && update-desktop-database "${APP_DIR}" || true
  command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q "${DATA_DIR}/icons/hicolor" || true
}

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -f "${BIN_DIR}/${NAME}" "${APP_DIR}/${NAME}.desktop" "${ICON_DIR}/${NAME}.svg"
  refresh_caches
  echo "Removed ${NAME}. Settings were kept in ~/.config/${NAME}."
  exit 0
fi

mkdir -p "${BIN_DIR}" "${APP_DIR}" "${ICON_DIR}"
install -m 755 "${HERE}/${NAME}" "${BIN_DIR}/${NAME}"
install -m 644 "${HERE}/${NAME}.svg" "${ICON_DIR}/${NAME}.svg"
# Absolute Exec path, so the launcher works even when ~/.local/bin isn't on PATH.
sed "s|^Exec=${NAME}|Exec=${BIN_DIR}/${NAME}|" "${HERE}/${NAME}.desktop" > "${APP_DIR}/${NAME}.desktop"
chmod 644 "${APP_DIR}/${NAME}.desktop"
refresh_caches

echo "Installed ${NAME} to ${BIN_DIR}."
case ":${PATH}:" in
  *":${BIN_DIR}:"*) ;;
  *) echo "Note: ${BIN_DIR} is not on your PATH. The app menu entry works anyway." ;;
esac
