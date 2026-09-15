# ARQHIA — WEB.md (sitio del producto)

> **Naturaleza: proyecto aparte, NO versionado dentro de ARQHIA.** Es una
> **dependencia/prerrequisito** de la versión estable v1.0. No ocupa un `v0.x`
> en `VERSIONS.md`; se desarrolla en paralelo (antes de v1.0) y su estado se
> sigue aquí.

## 1. Objetivo

Dar al producto: descarga, precios, soporte, documentación, changelog, páginas
legales y el host del manifiesto de actualizaciones. Además, la **Microsoft
Store exige una URL pública de política de privacidad y de soporte**, por lo
que la web es condición para publicar Windows.

## 2. Stack y ubicación

*   **Repo/carpeta propio:** `ARQHIA_WEB/` (hermano de `ARQHIA_P/`, sin
    mezclarse con el código de la app).
*   **Framework:** **Astro** (sitio estático, contenido en Markdown/`.astro`).
*   **Hosting:** **Cloudflare Pages** (gratis, HTTPS, CDN).
*   **Backend:** ninguno. El pago lo gestiona un proveedor externo
    (Lemon Squeezy / Paddle / Gumroad) con su propia página; la web solo enlaza.
*   **Actualizaciones:** el manifiesto es un JSON estático servido por la web.

## 3. Páginas

| Página | Contenido |
|---|---|
| **Landing** (`/`) | Propuesta de valor, capturas, CTA `Descargar` y `Pro`. |
| **Descargas** (`/download`) | Linux `.deb` / `.tar.gz` + checksums; Windows vía Microsoft Store. |
| **Precios** (`/pricing`) | Free vs Pro; enlaces de pago externos; Trial. |
| **Changelog** (`/changelog`) | Generado desde `VERSIONS.md`/release notes. |
| **Docs** (`/docs`) | Guía de inicio, configuración de API, permisos, FAQ. |
| **Soporte** (`/support`) | Email / foro / Discord + FAQ + issues. |
| **Privacidad** (`/privacy`) | Requisito de Microsoft Store. |
| **EULA/ToS** (`/terms`) | Términos de uso y licencia. |
| **Reembolsos** (`/refunds`) | Política de devoluciones. |

## 4. Manifiesto del updater

La app consulta este JSON al arrancar y compara con su versión (semver):

```json
{
  "version": "1.0.3",
  "channel": "stable",
  "notes_url": "https://arqhia.app/changelog#v1.0.3",
  "linux": {
    "format": "deb",
    "url": "https://arqhia.app/dl/arqhia_1.0.3_amd64.deb",
    "sha256": "<hash>"
  },
  "windows": { "store": true },
  "published_at": "2026-01-01T00:00:00Z"
}
```

*   Comportamiento v1.0: **notificar** (banner en Home con versión + novedades +
    enlace de descarga). No auto-reemplaza en esta fase.
*   Windows distribuido por Store: la Store actualiza sola y el aviso se puede
    omitir si el canal es Store.

## 5. Requisitos cruzados

*   Dominio propio + HTTPS.
*   `privacy` y `support` públicos (bloqueante para Microsoft Store).
*   `updates/latest.json` actualizado en cada release (script de release).
*   Changelog coherente con `VERSIONS.md` del producto.

## 6. Criterio Done

*   Web publicada en dominio propio con HTTPS.
*   Descargas de Linux y enlace a Microsoft Store operativos.
*   Privacidad, EULA y reembolsos publicados.
*   `updates/latest.json` versionado y servido correctamente.

## 7. Dependencias

*   Precede a **v1.0** (release estable). Sin web no hay publicación en
    Microsoft Store ni updater.
*   Depende de que existan binarios/paquetes de v0.9.6 (`.deb` + CI + icono).
