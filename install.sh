#!/usr/bin/env bash
# Installs the release build for the current user: binary, desktop entry, icons and the PDF file association.
# Run `pnpm tauri build` first, or pass --build to do it here. Pass --default to make Paper Wasp the default PDF app.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bin_src="$here/src-tauri/target/release/paper-wasp"
bin_dir="${HOME}/.local/bin"
app_dir="${HOME}/.local/share/applications"
icon_root="${HOME}/.local/share/icons/hicolor"
make_default=0

for arg in "$@"; do
  case "$arg" in
    --build) (cd "$here" && pnpm tauri build) ;;
    --default) make_default=1 ;;
    *) echo "Unknown option: $arg" >&2; exit 1 ;;
  esac
done

if [[ ! -x "$bin_src" ]]; then
  echo "No release binary at $bin_src. Run: pnpm tauri build   (or: $0 --build)" >&2
  exit 1
fi

mkdir -p "$bin_dir" "$app_dir"
install -m 755 "$bin_src" "$bin_dir/paper-wasp"

for size in 32 64 128 256 512; do
  mkdir -p "$icon_root/${size}x${size}/apps"
  if command -v magick >/dev/null 2>&1; then
    magick -background none "$here/src-tauri/app-icon.png" -resize "${size}x${size}" "$icon_root/${size}x${size}/apps/paper-wasp.png"
  else
    cp "$here/src-tauri/icons/128x128.png" "$icon_root/${size}x${size}/apps/paper-wasp.png"
  fi
done
mkdir -p "$icon_root/scalable/apps"
cp "$here/src/lib/assets/wasp-mark.svg" "$icon_root/scalable/apps/paper-wasp.svg"

cat > "$app_dir/paper-wasp.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Paper Wasp
Comment=A PDF reader that doesn't trust the document
Exec=$bin_dir/paper-wasp %f
Icon=paper-wasp
Terminal=false
Categories=Office;Viewer;
MimeType=application/pdf;
StartupWMClass=paper-wasp
StartupNotify=true
DESKTOP

update-desktop-database "$app_dir" 2>/dev/null || true
gtk-update-icon-cache -q -t "$icon_root" 2>/dev/null || true
if [[ $make_default -eq 1 ]]; then
  xdg-mime default paper-wasp.desktop application/pdf 2>/dev/null || true
fi

echo "Installed $bin_dir/paper-wasp with desktop entry $app_dir/paper-wasp.desktop"
if [[ $make_default -eq 1 ]]; then
  echo "Paper Wasp is now the default application for PDFs."
else
  echo "PDFs offer Paper Wasp under 'Open with'. Re-run with --default to make it the default."
fi
