# ARQHIA — Arquitectura del código

> Regla de capas (v0.9.5, ver `CONTEXT/PROJECT.md` §3): `views/` no toca DB
> ni red; `handlers/` coordina vía `app/` + dominio (`db`, `llm`, `agent`,
> `workspace`); los providers y las tools no conocen el estado UI.

```
ARQHIA_P/src/
├── main.rs            # shell: update/view guards + iced::application
├── app/
│   ├── events.rs      # Message / View / ConfigTab (solo datos)
│   ├── state.rs       # App + carga inicial + helpers puros
│   ├── history.rs     # ChatHistory: paralelos alineados (v0.9.5)
│   ├── projects.rs    # crear / papelera / borrado total (SQLite + FS)
│   ├── orchestrator.rs# driver planner → workers → auditor + run_fix_cycle
│   └── handlers/      # brazos de update por dominio
│       ├── chat.rs            # dispatch (SendPressed, modos, undo…)
│       ├── chat_stream.rs     # turnos + streaming (split v0.9.5)
│       ├── chat_history.rs    # project_key, spawn_chat, scroll (split v0.9.5)
│       ├── agent.rs           # plan/workers/permisos/auditoría (despacha)
│       └── {navigation,projects,config,questionnaire,stack}.rs
├── views/             # render puro (home, sidebar, chat, questionnaire…)
│   ├── config_view.rs # dispatch por pestaña
│   ├── config_api.rs  # pestaña API/perfiles (split v0.9.5)
│   └── config_git.rs  # pestaña Git (split v0.9.5)
├── agent/
│   ├── mod.rs         # plan_tasks, llm_step, exec_calls, auditor, analista
│   ├── roles.rs       # 5 roles + FixDecision + run_fix_cycle (v0.9.1/v0.9.5)
│   └── tools.rs       # tools + ExecPolicy + allowlist bash (sin shell)
├── llm/               # providers nativos (openai/anthropic/openrouter/local)
├── questionnaire/     # wizard + import + planning (MVP auto)
├── stack/ + skills.rs # STACK local FTS5 + skills embebidas
├── db.rs              # SQLite (chats, projects, messages, stack, uso)
├── workspace.rs       # guards + uploads + context_block (+ async v0.9.5)
├── paths.rs           # datos/config/home (`directories` + ARQHIA_HOME)
├── git.rs             # repo/rama/commit/push (async v0.9.5) + push (v0.7.2)
├── mcp.rs             # cliente MCP stdio/HTTP (mínimo v0.9.3)
├── pricing.rs         # catálogo models.dev + cache
└── config.rs / titles.rs / ui/
```

## Flujo de un turno Work

```
SendPressed → start_turn → begin_analysis_turn (placeholder + AgentAnalyze)
  → AgentAnalyze: brief → CONTEXT/ANALYSIS.md → planner (Net+Read con permiso)
  → AgentPlan: orch_tasks + start_worker(i)
  → AgentLlm: 1 paso LLM → calls? permiso? → spawn_exec_calls → AgentExecDone
  → continue_after_worker → … → auditor → AgentAudit
  → run_fix_cycle (orchestrator.rs, puro):
      CLEAN → finish_orchestrator → GitCommitDone (async) → GitPushDone?
      Fix   → AgentReanalyze (analista revisa TEMP) → worker de fixes → auditor…
      CapReached → finish sin commit
```

El handler (`handlers/agent.rs`) solo despacha `Message`; la decisión vive
en `orchestrator::run_fix_cycle` (testeable). El historial viaja en vectores
paralelos alineados por `App::history_*` (`history.rs`).

## Hilos y async

- La UI (Iced) nunca bloquea: git (`workspace_status_async`,
  `commit_all_async`), outlines (`code_outlines_async`) y lecturas
  (`context_block_async`, `scan_import_async`) corren en background
  (`tokio::process` o `spawn_blocking`) y resuelven en `Message::*`.
- El agente corre en `Task::perform` (planner → workers secuenciales en
  v0.x; paralelos 2–3 en v1.0 §F; sandboxes + merge en v1.1).
- Tokio adelgazado (v0.9.5): `rt-multi-thread, macros, fs, process, time,
  sync, io-util, io-std` (sin `full`, sin `net` propio: la red es reqwest).
