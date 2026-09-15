---
nombre: ui-ux
descripcion: Revisa UI con checklist visual (jerarquía, contraste, espaciados, densidades, estados) y escribe CONTEXT/UI-REVIEW.md.
version: 1.0
autor: ARQHIA
licencia: MIT
---

Revisa la interfaz que el usuario señale (pantalla, vista o flujo del workspace).

Checklist visual (en este orden):
1. Jerarquía: un foco claro por pantalla, títulos y acciones con peso coherente.
2. Contraste y color: texto legible, estados (ok/warn/error) distinguibles sin depender solo del color.
3. Espaciados: usa `design::gap/pad` (nada de números mágicos); aire entre bloques y respecto a bordes/scrollbars.
4. Densidades: cómoda vs compacta reducen espaciado de verdad (~20%); nada se aplasta ni desborda.
5. Responsive: anchos Fill vs fijos, pickers y botones sin desborde en ventana estrecha.
6. Estados vacíos y errores: cada lista/panel tiene su vacío amable y su error legible (sin pánicos ni pantallas en blanco).

Cierre obligatorio:
- Si tienes herramientas de escritura, guarda el reporte en `CONTEXT/UI-REVIEW.md` del workspace con este formato: `## Pantalla` + hallazgos numerados + propuesta concreta por hallazgo. Si no tienes herramientas, devuelve el reporte en el chat con el mismo formato.
- Termina citando el reporte (`CONTEXT/UI-REVIEW.md`) y las 3 mejoras de mayor impacto primero. Sin rodeos.
