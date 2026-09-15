# AGENTS.md — reglas del orquestador ARQHIA

## Roles
Los 5 roles del orquestador (v0.9.1; prompts y gates en `src/agent/roles.rs`):
- orquestador: dueño del turno; decide el plan global y cierra (veredicto).
- analista: lee `CONTEXT/` + specs y redacta el brief (`CONTEXT/ANALYSIS.md`); solo Read.
- planner: divide el brief en subtareas disjuntas (archivos distintos); solo Net+Read.
- worker (generador): ejecuta UNA subtarea con read/write/edit/delete/list/bash/search/fetch_url.
- auditor: solo lee y reporta a CONTEXT/TEMP.md. Nunca escribe código; veredicto CLEAN/ISSUES.

## Tools permitidas
read_file (paginado: offset/limit), get_file_outline, write_file, edit_file, delete_file, list_dir, search_files (bloques con contexto), bash (allowlist + `git` con política por subcomando), fetch_url (dominios).

## Reglas
- Rutas relativas al workspace. Nunca escribir fuera (guard estricto).
- `edit_file` exige 1 coincidencia exacta de `old`.
- Pasos LLM→tools y tareas del plan según Límites de config; installs y red piden permiso (Install/Net).
- Git (v0.7.2): el agente trabaja en la rama de trabajo (`ARQHIA`); la base (`main`/`master`) está protegida. Push pide aprobación; `push --force`, `reset --hard`, `clean`, `rebase`, `config` y `remote add/remove` están bloqueados.
- Tras workers, el auditor revisa (código + `cargo check`/`test`/`clippy`) y escribe CONTEXT/TEMP.md; el bucle `auditor → fix` repite hasta quedar verde (tope `Limits.max_fix_cycles`, 0 = ilimitado). El auto-commit solo ocurre con el turno verde (analista/planner: ver `CONTEXT/ANALYSIS.md`).
