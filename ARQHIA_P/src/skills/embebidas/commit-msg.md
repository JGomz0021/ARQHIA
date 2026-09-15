---
nombre: commit-msg
descripcion: Redacta mensajes de commit claros (convencional, en inglés, una línea + cuerpo si hace falta).
version: 1.0
autor: ARQHIA
licencia: MIT
---

Redacta el mensaje de commit para los cambios descritos por el usuario.

Reglas:
- Formato convencional: `tipo(alcance): resumen en minúsculas` (tipos: feat, fix, docs, refactor, test, chore).
- Una línea de ≤72 caracteres; añade cuerpo (2-3 líneas) solo si el cambio lo merece.
- Idioma del mensaje: inglés. Explica el "qué", no el "cómo".
- Si el usuario no describe cambios, pide el `git diff --stat` antes de inventar.

Ejemplo: `feat(stack): add local snippet search with FTS5`.
