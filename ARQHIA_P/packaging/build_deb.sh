#!/usr/bin/env bash
# ARQHIA — empaquetador .deb (v0.9.6, puerta de v1.0).
#
# Uso:  packaging/build_deb.sh
# Salida: packaging/dist/arqhia_<version>_<arch>.deb
#
# Requisitos: cargo, dpkg-deb. No usa cargo-deb (fallback sin red).
# El paquete NUNCA borra datos: `postinst` solo crea carpetas si faltan
# (ver CONTEXT/POLICIES.md §7).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
cd "$ROOT"

VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
ARCH="$(dpkg --print-architecture)"
PKG="arqhia"
STAGE="$HERE/dist/${PKG}_${VERSION}_${ARCH}"

echo "==> ARQHIA $VERSION ($ARCH)"
echo "==> cargo build --release (perfil: strip + lto)"
cargo build --release --locked

BIN="target/release/arqhia_p"
[ -x "$BIN" ] || { echo "ERROR: falta $BIN"; exit 1; }

rm -rf "$STAGE"
mkdir -p "$STAGE/DEBIAN" \
         "$STAGE/usr/bin" \
         "$STAGE/usr/share/applications" \
         "$STAGE/usr/share/metainfo" \
         "$STAGE/usr/share/doc/$PKG"

install -m 0755 "$BIN" "$STAGE/usr/bin/arqhia"
install -m 0644 "$HERE/arqhia.desktop" "$STAGE/usr/share/applications/arqhia.desktop"
install -m 0644 "$HERE/arqhia.metainfo.xml" "$STAGE/usr/share/metainfo/com.arqhia.ARQHIA.metainfo.xml"
install -m 0644 "$ROOT/assets/icon.svg" "$STAGE/usr/share/doc/$PKG/icon.svg"

for size in 16 32 64 128 256; do
  dir="$STAGE/usr/share/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$dir"
  install -m 0644 "$ROOT/assets/icon-${size}.png" "$dir/arqhia.png"
done

INSTALLED_SIZE="$(du -sk "$STAGE/usr" | cut -f1)"
cat > "$STAGE/DEBIAN/control" <<EOF
Package: $PKG
Version: $VERSION
Section: devel
Priority: optional
Architecture: $ARCH
Installed-Size: $INSTALLED_SIZE
Maintainer: ARQHIA <soporte@arqhia.dev>
Homepage: https://github.com/JGomz0021/ARQHIA
Description: Entorno de desarrollo con agente IA (Iced nativo)
 ARQHIA acompaña al usuario desde la definición de la idea hasta el
 proyecto estable: cuestionario, especificaciones, STACK local y agentes
 bajo permisos explícitos. Sin telemetría.
EOF

cat > "$STAGE/DEBIAN/postinst" <<'EOF'
#!/bin/bash
# Crea las carpetas de usuario si faltan. JAMÁS borra DB/config/workspaces
# (CONTEXT/POLICIES.md §7). Todo es best-effort: si no puede, no falla.
set -e

target_user="${SUDO_USER:-${PKEXEC_UID:+$(id -un "$PKEXEC_UID" 2>/dev/null)}}"
target_home=""
if [ -n "$target_user" ] && [ "$target_user" != "root" ]; then
    target_home="$(getent passwd "$target_user" | cut -d: -f6 || true)"
fi
if [ -z "$target_home" ] && [ -d /home ] && [ "$(id -u)" -eq 0 ]; then
    for d in /home/*; do
        [ -d "$d" ] && target_home="$d" && break
    done
fi

if [ -n "$target_home" ] && [ -d "$target_home" ]; then
    mkdir -p "$target_home/.config/arqhia" \
             "$target_home/.local/share/arqhia/skills" \
             "$target_home/.local/share/arqhia/backups" \
             "$target_home/ARQHIA/projects"
    # config.toml de defaults: SOLO si no existe (nunca pisa la del usuario).
    if [ ! -f "$target_home/.config/arqhia/config.toml" ]; then
        printf '%s\n' \
          '# ARQHIA — config.toml' \
          '# Este archivo lo completa la app en Configuración → API/Git.' \
          > "$target_home/.config/arqhia/config.toml"
    fi
    if [ -n "$target_user" ] && [ "$target_user" != "root" ]; then
        chown -R "$target_user" "$target_home/.config/arqhia" \
                               "$target_home/.local/share/arqhia" \
                               "$target_home/ARQHIA" 2>/dev/null || true
    fi
fi

# Refresca la caché de iconos/menú si las herramientas están disponibles.
command -v update-desktop-database >/dev/null 2>&1 && \
    update-desktop-database -q /usr/share/applications || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && \
    gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true

exit 0
EOF
chmod 0755 "$STAGE/DEBIAN/postinst"

mkdir -p "$HERE/dist"
DEB="$HERE/dist/${PKG}_${VERSION}_${ARCH}.deb"
dpkg-deb --build --root-owner-group "$STAGE" "$DEB"
rm -rf "$STAGE"

echo "==> OK: $DEB"
dpkg-deb --info "$DEB" | sed -n '1,20p'
