# ADR-003 — Una rama de trabajo (`ARQHIA`), base protegida

- Estado: aceptado (v0.7.2)
- Fecha: 2026 (revisión v0.9.5)

## Contexto

El agente commitea por el usuario; hay que separar su trabajo del código
estable y de ediciones manuales ajenas.

## Decisión

Cada workspace es un repo; el agente trabaja **solo en `ARQHIA`**
(`BranchMode::Single` por defecto). `main`/`master` protegidas: ningún
comando del agente commitea sobre ellas. Guarda anti-sucio (no commitea si
el árbol venía sucio), puerta de calidad (`check+test+clippy`) y push OFF +
aprobación. `BranchMode::PerTask` existe pero no se recomienda hasta v1.1
(sandboxes + merge).

## Consecuencias

- (+) Historial legible (`ARQHIA: <resumen>`), base siempre estable.
- (+) Sin conflictos en v0.x (secuencial sobre una rama).
- (−) Un solo hilo de trabajo por workspace hasta v1.1.
