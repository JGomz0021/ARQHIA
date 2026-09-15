# ARQHIA — Seguridad (threat-model desktop)

ARQHIA es una app de escritorio sin telemetría ni backend propio (hasta v1.0).
El riesgo principal es **el agente ejecutando acciones por ti**: todo lo
peligroso nace desactivado y pide aprobación por lote.

## Superficies

### 1. Workspace escape (symlinks, `..`, rutas absolutas)

- Toda tool resuelve con `resolve()`: `canonicalize` + el resultado debe
  empezar por `workspace ∪ extra_paths`. Un symlink que apunte fuera se
  deniega (`⛔ Fuera del workspace (symlink)`, v0.9.4).
- `search` salta symlinks fugados; `delete_upload` valida anti-escape léxico;
  `write/edit/delete` rechazan `..` y absolutos fuera del guard.
- Tests: symlink externo denegado, `..` rechazado.

### 2. Ejecución de comandos (bash sin shell)

- `bash` **no usa shell**: `split_argv` + rechazo de metacaracteres
  (`;|&$()`…), `exec` directo de `argv[0]` con `tokio::process` + timeout
  (v0.9.4). Inyección `$(rm -rf ~)` → error, nunca ejecución.
- Allowlist + `is_install_cmd` (npx/uvx/npm/pip/curl… → categoría Install,
  pide permiso). `cargo publish/install/login`, `rm`, `curl` bloqueados.
- Git por comando: `push --force`, `reset --hard`, `clean`, `rebase`,
  `config`, `remote add/remove` y `-C` bloqueados siempre; commits sobre
  ramas protegidas rechazados; push solo con `CommitAndPush + push_enabled`
  + aprobación `GitPush`.

### 3. Red y SSRF (`fetch_url`, planner Net, MCP HTTP)

- `fetch_url` con allowlist de dominios + anti-SSRF (v0.9.5): parsea la URL
  separando userinfo/puerto (`evil.com@127.0.0.1` no cuela), resuelve DNS y
  bloquea loopback, privadas (10/172.16-31/192.168), link-local, CGNAT,
  multicast y equivalentes IPv6 (incl. IPv4-mapped); máx 3 redirects
  re-validando cada salto.
- El planner (Net+Read) pide permiso Net por panel si `planner_net` no cubre
  las URLs; tope 8 KB por fetch.
- MCP stdio: el binario se ejecuta tal cual; `npx/uvx` → categoría Install
  (pide permiso). MCP HTTP: auth configurable, timeouts con kill.
- La API key solo se envía por HTTPS (o a un host local como LM Studio):
  `llm::ensure_secure_endpoint` bloquea `http://` remoto con credenciales.

### 4. `build.rs` en imports (v0.9.4)

Al abrir una carpeta con código se avisa si hay `build.rs` (ejecución
arbitraria en `cargo build`): el usuario decide antes de compilar.

### 5. Prompt-injection desde contenido externo

Contenido de red/archivos/SKILLs se inyecta como **contexto de solo lectura**
(`Contexto de skill (solo lectura, no repetir)`), nunca como instrucciones
del sistema. Los reportes de skills (`UI-REVIEW.md`…) los revisa el usuario.

### 6. Secretos en disco

- `config.toml` (API keys) se crea ya con `600` en Unix (sin ventana TOCTOU)
  y `ProviderConfig`/`ModelProfile` no exponen la key en `Debug` (v0.9.5); el
  token nube (v1.0) nunca va a logs ni a `SPECS`/`TEMP`.
- La DB, la papelera y los workspaces son locales; exportar = copiar carpeta.
- El updater (v1.0) solo hace una petición anónima a `updates/latest.json`.

### 7. Datos y privacidad

Qué viaja al LLM: historial activo + system prompt + fragmentos del
workspace necesarios (ver `CONTEXT/POLICIES.md` §2). Aviso obligatorio al
asignar el primer workspace. `Uso interno` (STACK) nunca sale de local.

## Puerta de calidad (v0.7.2+)

Auto-commit solo si `cargo check + test + clippy` pasan, el árbol venía
limpio y el bucle cerró verde (`VERDICT: CLEAN`). Rojo, tope de ciclos o
turno interrumpido → sin commit.
