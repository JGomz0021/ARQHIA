# AGENTS.md — reglas del orquestador ARQHIA

## Roles
- planner: divide el pedido en subtareas disjuntas (archivos distintos).
- worker (generador): ejecuta UNA subtarea con read/write/edit/delete/list/bash/search/fetch_url.
- auditor: solo lee y reporta a CONTEXT/TEMP.md. Nunca escribe código.

## Tools permitidas
read_file, write_file, edit_file, delete_file, list_dir, search_files, bash (allowlist), fetch_url (dominios).

## Reglas
- Rutas relativas al workspace. Nunca escribir fuera (guard estricto).
- `edit_file` exige 1 coincidencia exacta de `old`.
- Pasos LLM→tools y tareas del plan según Límites de config; installs y red piden permiso (Install/Net).
- Tras workers, el auditor revisa y escribe CONTEXT/TEMP.md; si hay issues, 1 pasada de fixes.
