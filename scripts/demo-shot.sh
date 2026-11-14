#!/usr/bin/env bash
# Captura a janela do modo demo numa sessão headless, sem abrir nada na área de
# trabalho. Uso: bash scripts/demo-shot.sh SAIDA.png [ARGUMENTOS DA DEMO...]
# Exemplo: bash scripts/demo-shot.sh .cache/updates.png --demo-tab updates --theme dark
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
(($#)) || { echo "Uso: $0 SAIDA.png [ARGUMENTOS DA DEMO...]" >&2; exit 2; }
if [[ ${HEADLESS_SESSION:-0} != 1 ]]; then
  cargo build --locked
  exec bash scripts/headless.sh bash scripts/demo-shot.sh "$@"
fi
command -v grim >/dev/null || { echo 'Instale grim: sudo pacman -S grim' >&2; exit 1; }
out=$1
shift
mkdir -p "$(dirname -- "$out")"
"${CARGO_TARGET_DIR:-target}/debug/pacmanager" --demo "$@" &
app=$!
trap 'kill "$app" 2>/dev/null || true; wait "$app" 2>/dev/null || true' EXIT
rect=''
for _ in {1..100}; do
  kill -0 "$app" 2>/dev/null || { echo 'PacManager encerrou antes de abrir a janela' >&2; exit 1; }
  rect=$(swaymsg -t get_tree | python3 -c '
import json, sys
def walk(node):
    yield node
    for child in node.get("nodes", []) + node.get("floating_nodes", []):
        yield from walk(child)
for node in walk(json.load(sys.stdin)):
    if node.get("pid") == int(sys.argv[1]):
        r = node["rect"]
        print("%d,%d %dx%d" % (r["x"], r["y"], r["width"], r["height"]))
        break
' "$app")
  [[ -n "$rect" ]] && break
  sleep 0.1
done
[[ -n "$rect" ]] || { echo 'A janela do PacManager não abriu' >&2; exit 1; }
# Espera ícones e o layout assentarem.
sleep "${SHOT_DELAY:-2}"
grim -g "$rect" "$out"
echo "Captura salva em $out"
