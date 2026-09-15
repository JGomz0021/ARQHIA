# ARQHIA — VERSIONS.md

Índice de estado por versión. Detalle en `CONTEXT/VERSIONS/v0.x.md`.
Dependencias no versionadas (web) en `CONTEXT/WEB.md`.

| Versión | Nombre | Estado | Link |
|---|---|---|---|
| v0.1 | Chat normal multi-provider (Iced, sin agente) | 🟢 Done | [v0.1.md](VERSIONS/v0.1.md) |
| v0.2 | Sidebar + temas + proyectos agrupadores | 🟢 Done | [v0.2.md](VERSIONS/v0.2.md) |
| v0.3 | Agente simple (workspace + files + bash) | 🟢 Done | [v0.3.md](VERSIONS/v0.3.md) |
| v0.4 | Home page (crear/abrir/config) | 🟢 Done | [v0.4.md](VERSIONS/v0.4.md) |
| v0.5 | Cuestionario simple → doc de especificaciones | 🟢 Done | [v0.5.md](VERSIONS/v0.5.md) |
| v0.6 | Orquestador + permisos + subagentes | 🟢 Done | [v0.6.md](VERSIONS/v0.6.md) |
| v0.7 | Polish + seguridad (tabs, uploads, permisos, límites, apariencia, POLICIES) | 🟢 Done | [v0.7.md](VERSIONS/v0.7.md) |
| v0.7.1 | Agente eficiente + Modos Chat/Plan/Work | 🟢 Done | [v0.7.1.md](VERSIONS/v0.7.1.md) |
| v0.7.2 | Git nativo + puerta de calidad | 🟢 Done | [v0.7.2.md](VERSIONS/v0.7.2.md) |
| v0.7.3 | Bucle de estabilidad (analista → workers → auditor → fix hasta verde → commit) | 🟢 Done | [v0.7.3.md](VERSIONS/v0.7.3.md) |
| v0.7.4 | Chat UX + modelos con nombre (perfiles, título IA, undo, copiar/bifurcar, timestamps, fuentes) | 🟢 Done | [v0.7.4.md](VERSIONS/v0.7.4.md) |
| v0.8 | Cuestionario genérico + por nivel + IA, estructura de proyecto, onboarding y sesiones | 🟢 Done | [v0.8.md](VERSIONS/v0.8.md) |
| v0.8.1 | Cuestionario universal (Categoría+Tipo, condicional, licencias) + import auto | 🟢 Done | [v0.8.1.md](VERSIONS/v0.8.1.md) |
| v0.8.2 | Cuestionario reordenado + matriz profunda + OSS + pantalla de carga MVP | 🟢 Done | [v0.8.2.md](VERSIONS/v0.8.2.md) |
| v0.9 | STACK local + legalidad + identidad (sin instalador) + infra skills + contexto auto post-cuestionario | 🟢 Done | [v0.9.md](VERSIONS/v0.9.md) |
| v0.9.1 | BETA agentes (Orquestador/Analista/Planner Net+Read/Workers/Auditor + loop) | 🟢 Done | [v0.9.1.md](VERSIONS/v0.9.1.md) |
| v0.9.2 | Skills de dominio (UI/UX, CodeReview, Test/QA → CONTEXT/) | 🟢 Done | [v0.9.2.md](VERSIONS/v0.9.2.md) |
| v0.9.3 | MCP mínimo (stdio + HTTP, tools como `mcp__srv__tool`) | 🟢 Done | [v0.9.3.md](VERSIONS/v0.9.3.md) |
| v0.9.4 | Hardening crítico (anti-symlink + FKs/índices + tests en temp + `directories` + 600) | 🟢 Done | [v0.9.4.md](VERSIONS/v0.9.4.md) |
| v0.9.5 | Calidad estructural (desacople `App` + git async + docs + `tracing`) | 🟢 Done | [v0.9.5.md](VERSIONS/v0.9.5.md) |
| v0.9.6 | Revisión + optimización + refactor + tests + icono + instalador Linux | 🟢 Done (puerta de v1.0) | [v0.9.6.md](VERSIONS/v0.9.6.md) |
| v1.0 | STACK nube + auth + updater + workers async + release estable (Linux+Windows) | ☐ Pendiente | [v1.0.md](VERSIONS/v1.0.md) |
| v1.0.1 | MCP avanzado (pestaña, resources/prompts, item `MCP`, auth HTTP) | ☐ Pendiente | [v1.0.1.md](VERSIONS/v1.0.1.md) |
| v1.1 | Pro (Linux+Windows): sandboxes + merge + agentes/flujos + licencias | ☐ Pendiente | [v1.1.md](VERSIONS/v1.1.md) |
| v1.2 | macOS: firma Developer ID + notarización (condicionado a 50–100 PRO) | ☐ Pendiente | [v1.2.md](VERSIONS/v1.2.md) |

## Dependencias no versionadas

| Proyecto | Nombre | Estado | Link |
|---|---|---|---|
| WEB | Sitio del producto (Astro + Cloudflare): descarga, precios, soporte, legal, updater manifest | ☐ Pendiente | [WEB.md](WEB.md) |

Estados: ☐ Pendiente | 🟡 En curso | 🟢 Done | 🔴 Bloqueado
