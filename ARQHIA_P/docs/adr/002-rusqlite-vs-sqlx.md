# ADR-002 — rusqlite embebido en vez de sqlx/servidor

- Estado: aceptado (v0.1)
- Fecha: 2026-01 (revisión v0.9.5)

## Contexto

Persistencia local de chats, proyectos, mensajes, STACK (FTS5) y uso, sin
cuentas ni servidor hasta v1.0.

## Decisión

**rusqlite con `bundled`** (SQLite compilado dentro, incluye FTS5).
Conexión por operación + `busy_timeout` 5s + `PRAGMA foreign_keys=ON`.
Migraciones aditivas idempotentes (`ADD COLUMN` tolerante, tablas nuevas +
backfill, nunca destructivas).

## Consecuencias

- (+) Cero servicios, cero setup; la DB es un fichero portable.
- (+) FTS5 para el STACK sin dependencias extra.
- (−) Sin pool async: cada `connect()` abre el fichero (barato en local).
- (−) La nube (v1.0) será otro cliente (`stack_cloud.rs` + mock), no la misma DB.
