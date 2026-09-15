#!/usr/bin/env bash
# ARQHIA — instalador genérico (fallback sin root, v0.9.6).
#
# Uso:
#   packaging/install.sh                 # ~/.local (sin root)
#   sudo packaging/install.sh            # /usr/local (sistema)
#
# Construye en release si hace falta y copia binario + iconos + .desktop.
# JAMÁS borra DB/config/workspaces (CONTEXT/POLICIES.md §7).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
cd "$ROOT"

if [ -n "${PREFIX:-}" ]; then
    :
elif [ "$(id -u)" -eq 0 ]; then
    PREFIX="/usr/local"
else
    PREFIX="$HOME/.local"
fi

echo "==> ARQHIA → $PREFIX"
[ -x target/release/arqhia_p ] || cargo build --release --locked

install -d "$PREFIX/bin" \
           "$PREFIX/share/applications" \
           "$PREFIX/share/metainfo" \
           "$PREFIX/share/doc/arqhia"
install -m 0755 target/release/arqhia_p "$PREFIX/bin/arqhia"
install -m 0644 "$HERE/arqhia.desktop" "$PREFIX/share/applications/arqhia.desktop"
install -m 0644 "$HERE/arqhia.metainfo.xml" "$PREFIX/share/metainfo/com.arqhia.ARQHIA.metainfo.xml"

for size in 16 32 64 128 256; do
    dir="$PREFIX/share/icons/hicolor/${size}x${size}/apps"
    install -d "$dir"
    install -m 0644 "$ROOT/assets/icon-${size}.png" "$dir/arqhia.png"
done

# Carpetas de datos del usuario (si no existen). Nunca pisa nada.
mkdir -p "$HOME/.config/arqhia" \
         "$HOME/.local/share/arqhia/skills" \
         "$HOME/.local/share/arqhia/backups" \
         "$HOME/ARQHIA/projects"
if [ ! -f "$HOME/.config/arqhia/config.toml" ]; then
    printf '%s\n' \
      '# ARQHIA — config.toml' \
      '# Este archivo lo completa la app en Configuración → API/Git.' \
      > "$HOME/.config/arqhia/config.toml"
fi

echo "==> Listo. Ejecuta: $PREFIX/bin/arqhia"
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) echo "    (añade $PREFIX/bin al PATH para lanzar 'arqhia')" ;;
esac
