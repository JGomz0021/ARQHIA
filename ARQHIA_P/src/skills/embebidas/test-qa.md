---
nombre: test-qa
descripcion: Ejecuta check/test/clippy, propone pruebas personalizadas y escribe CONTEXT/QA-REPORT.md con tabla pass/fail.
version: 1.0
autor: ARQHIA
licencia: MIT
---

Asegura la calidad del workspace que el usuario señale (o el actual si no señala nada).

Protocolo (en este orden):
1. Puerta base: `cargo check`, `cargo test`, `cargo clippy --all-targets` (timeout amplio). Lee el Log del turno para no repetir lo ya verificado.
2. Tabla de resultados: un comando por fila con `pass/fail` + cola del output (últimas 20 líneas bastan).
3. Foco: si algo falla, reproduce en mínimo (1 test o 1 comando) y propone el fix concreto; no reescribas medio proyecto.
4. Pruebas personalizadas: propón 1–3 pruebas nuevas que cubran el hueco (happy path + borde + regresión del fallo). Solo las EJECUTES si tienes permiso de escritura/consola; si no, déjalas propuestas con el comando exacto.
5. Sin scripts propios: cualquier script o comando pasa por el permiso Bash normal del usuario.

Cierre obligatorio:
- Si tienes herramientas de escritura, guarda el reporte en `CONTEXT/QA-REPORT.md` del workspace con este formato: `## Puerta base` (tabla comando | pass/fail | nota) + `## Propuestas` (test + comando). Si no tienes herramientas, devuelve el reporte en el chat con el mismo formato.
- Termina citando el reporte (`CONTEXT/QA-REPORT.md`) y el veredicto en una línea (`QA: PASS` o `QA: FAIL — <causa>`). Sin rodeos.
