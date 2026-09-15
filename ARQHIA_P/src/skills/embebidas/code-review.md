---
nombre: code-review
descripcion: Revisa código con foco en seguridad y deuda, y escribe CONTEXT/CODE-REVIEW.md con severidad por archivo:línea.
version: 1.0
autor: ARQHIA
licencia: MIT
---

Revisa el código del workspace que el usuario señale (o el último diff si no señala nada).

Checklist (en este orden):
1. Seguridad: inyección por concatenación en comandos (bash con `sh -c` vs `exec` directo), escapes del workspace (rutas `../`, absolutas y **symlinks que apuntan fuera**), secretos o API keys en logs/mensajes, `fetch_url` sin allowlist (SSRF: localhost, 169.254.169.254).
2. Robustez: `unwrap`/`expect` en producción (solo admisibles en tests), `todo!/unimplemented!` pendientes, errores con mensaje visible en español.
3. Correctness: supuestos rotos entre módulos, estados paralelos desalineados (mensajes vs markdown vs uso), ramas que nunca se ejecutan.
4. Deuda: duplicados que piden fuente única, `ChatHistory`/paralelos que se rellenan a mano en varios sitios, funciones con demasiados argumentos.
5. Alternativas: para cada Alta, una alternativa más simple si existe.

Cierre obligatorio:
- Si tienes herramientas de escritura, guarda el reporte en `CONTEXT/CODE-REVIEW.md` del workspace con este formato: una línea por issue `- [Alta|Media|Baja] archivo:línea — qué + arreglo propuesto`. Si no tienes herramientas, devuelve el reporte en el chat con el mismo formato.
- Termina citando el reporte (`CONTEXT/CODE-REVIEW.md`); 0 Altas se dice explícito (`CLEAN`). Sin rodeos.
