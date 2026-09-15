# ARQHIA — Configuración

Todo vive en `AppConfig` (`src/config.rs`), serializado como TOML. Sin
migraciones destructivas: cada campo nuevo lleva `#[serde(default)]`.

## Rutas

| Fichero | Ruta por defecto | Override |
|---|---|---|
| `config.toml` | `~/.config/arqhia/config.toml` | `$ARQHIA_CONFIG` |
| `arqhia.db` | `~/.local/share/arqhia/arqhia.db` | `$ARQHIA_DB` (o `$ARQHIA_HOME`) |
| `models.dev.json` (cache precios) | `~/.local/share/arqhia/models.dev.json` | — |
| `skills/` | `~/.local/share/arqhia/skills/` | — |
| `papelera/` | `~/.local/share/arqhia/papelera/` | — |

Resolución vía `src/paths.rs` (`directories::ProjectDirs` + `ARQHIA_HOME`
para tests/agente). En Unix `config.save()` deja el fichero en `600`.

## Ejemplo (`config.toml`)

```toml
active = "OpenAI"
theme = "Dark"

[openai]
provider = "OpenAI"
api_key = "sk-..."
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
reasoning_effort = "auto"

[local]
provider = "Local"
api_key = ""
base_url = "http://localhost:1234"
model = "qwen3-8b"   # id exacto cargado en LM Studio

[permissions]
auto_read = true
auto_write = false   # lo peligroso nace desactivado
auto_bash = false
auto_net = false
planner_net = false  # el planner pide Red por panel (v0.9.1)
auto_install = false
net_domains = ["models.dev"]

[limits]
max_iters = 10
max_tasks = 6
max_read_kb = 200
bash_timeout_s = 30
max_tokens_turn = 0  # 0 = ilimitado
max_fix_cycles = 0   # 0 = ilimitado (v0.7.3)

[git]
enabled = true
auto_init = true
base_branch = "main"
work_branch = "ARQHIA"
autonomy = "CommitLocal"  # ReadOnly | CommitLocal | CommitAndPush
push_enabled = false
remote = "origin"

[stack_consent]
use_stack = false
share_local = false
share_cloud = false

[mcp.servers.ejemplo]
transport = "Stdio"
command = "npx"
args = ["-y", "mi-servidor-mcp"]
auto = false
timeout_s = 30
```

## Perfiles de modelo (v0.7.4)

Además del modelo activo por provider, `model_profiles` guarda perfiles con
nombre visible (`provider + base_url + api_key + model + nivel`). El selector
del composer y de Config lista perfiles por nombre; `active_profile` manda.

## Notas

- `has_any_api()`: Local cuenta configurado con solo el modelo (sin key).
- El catálogo models.dev se descarga solo al arrancar si falta (una vez,
  cacheado) para niveles de razonamiento, contexto y costes.
- Límites con rangos validados (`Limits::clamped()`); tooltips en la UI.
