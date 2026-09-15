# ADR-004 — Workers secuenciales → async (v1.0), sandboxes (v1.1)

- Estado: aceptado (diseño; ejecución en v1.0/v1.1)
- Fecha: 2026 (revisión v0.9.5)

## Contexto

v0.6–v0.9 ejecutan workers **secuenciales** sobre `Project/` (simple y sin
conflictos, pero lento). El plan Pro promete paralelismo real.

## Decisión

1. **v0.9.5 (esta versión):** sin cambiar el modelo de ejecución, todo lo
   bloqueante sale de la UI (`workspace_status_async`, `commit_all_async`,
   `code_outlines_async`, `spawn_blocking`). La UI responde aunque el repo
   sea grande; el auditor sigue secuencial.
2. **v1.0 §F:** 2–3 workers async sobre tareas **disjuntas** (el planner
   particiona por archivos) + fallback secuencial, 1 commit.
3. **v1.1:** sandboxes `{ws}/.arqhia/work/task-N/` + `merge.rs` file-level
   (`MERGE.md` + auditor si hay conflicto), hasta 8 workers (Pro).

## Consecuencias

- (+) v0.9.5 no arriesga estabilidad: mismo comportamiento, mejor latencia.
- (+) El paralelismo v1.0 reutiliza `run_fix_cycle` y la puerta de calidad.
- (−) El merge real espera a v1.1; hasta entonces, tareas solapadas = secuencial.
