#!/bin/sh
set -eu

source_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
binary="$source_dir/pacmanager"
if [ ! -f "$binary" ]; then
    binary="$source_dir/target/release/pacmanager"
fi
if [ ! -f "$binary" ]; then
    echo 'Compile primeiro com cargo build --locked --release.' >&2
    exit 1
fi
bin_dir="$HOME/.local/bin"
data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"
install -Dm755 "$binary" "$bin_dir/pacmanager"
install -Dm644 "$source_dir/assets/icons/pacmanager.svg" "$data_dir/icons/hicolor/scalable/apps/pacmanager.svg"
install -Dm644 "$source_dir/assets/icons/pacmanager.png" "$data_dir/icons/hicolor/256x256/apps/pacmanager.png"
install -Dm644 "$source_dir/assets/pacmanager.desktop" "$data_dir/applications/pacmanager.desktop"
# Desktop launchers do not always inherit the user's shell PATH.
python3 - "$data_dir/applications/pacmanager.desktop" "$bin_dir/pacmanager" <<'PY'
from pathlib import Path
import sys
p = Path(sys.argv[1])
executable = sys.argv[2].replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$').replace('%', '%%')
p.write_text(p.read_text().replace('Exec=pacmanager', 'Exec="' + executable + '"'))
PY
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$data_dir/applications"
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache --force --ignore-theme-index "$data_dir/icons/hicolor"
fi
touch "$data_dir/icons/hicolor"
echo "PacManager instalado em $bin_dir/pacmanager"
