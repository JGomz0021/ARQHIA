# ARQHIA

ARQHIA es un agente de IA nativo que acompaña al usuario desde la definición
de la idea hasta la resolución estable del proyecto. A diferencia de otros
asistentes que solo generan código, ARQHIA ayuda a diseñar el proyecto, sus
especificaciones, funcionalidades y stack, adaptándose al nivel de experiencia
del usuario y trabajando de forma delegable y estable paso a paso.

## Quickstart

Requisitos: Rust estable (1.85+; el crate usa `edition = "2024"`) y, en Linux,
las dependencias de Iced (`pkg-config`, `libxkbcommon`, Wayland/X11 u OpenGL
según tu sesión).

```sh
git clone <este-repo>
cd ARQHIA/ARQHIA_P
cargo run
```

La app abre en Home. Si es tu primer arranque verás el aviso de bienvenida:

1. Ve a **Configuración → API** (o pulsa el aviso).
2. **Opción nube:** elige provider (OpenAI, Anthropic u OpenRouter), pega tu
   API key y elige modelo (botón `Buscar modelo` para ver precios de
   models.dev). Pulsa **Guardar perfil** y **Probar conexión**.
3. **Opción local (sin key):** abre [LM Studio](https://lmstudio.ai), carga un
   modelo y anota su id exacto. En ARQHIA elige provider `Local (LM Studio)`
   con `http://localhost:1234` y ese id como modelo.
4. Crea un proyecto en Home y chatea. Los modos **Chat / Plan / Work** están
   en el composer (`Ctrl+1/2/3`).

## Instalación (Linux, v0.9.6)

```sh
# .deb (Debian/Ubuntu) — instala /usr/bin/arqhia + icono + .desktop
ARQHIA_P/packaging/build_deb.sh
sudo dpkg -i ARQHIA_P/packaging/dist/arqhia_*.deb

# Genérico sin root (a ~/.local) o con PREFIX=/usr/local
ARQHIA_P/packaging/install.sh
```

El instalador **nunca borra** tu `config.toml`, tu `arqhia.db` ni tus
workspaces; solo crea carpetas si faltan (`CONTEXT/POLICIES.md` §7). El
empaquetado y el chequeo de calidad (`fmt` + `clippy -D warnings` + `test` +
`build`) corren en `.github/workflows/ci.yml`.

## Rutas de datos

| Qué | Dónde |
|---|---|
| Config (`config.toml`, API keys, tema, permisos, git) | `~/.config/arqhia/` (o `$ARQHIA_CONFIG`) |
| Base de datos (`arqhia.db`: chats, proyectos, STACK, uso) | `~/.local/share/arqhia/` (o `$ARQHIA_DB` / `$ARQHIA_HOME`) |
| Catálogo de modelos/precios (cache models.dev) | `~/.local/share/arqhia/models.dev.json` |
| Skills locales | `~/.local/share/arqhia/skills/` |
| Papelera de proyectos borrados | `~/.local/share/arqhia/papelera/` |

La config se guarda con permisos `600` en Unix. No hay telemetría ni cuentas:
todo vive en tu equipo (ver `CONTEXT/POLICIES.md`).

## Desarrollo

```sh
cd ARQHIA_P
cargo test        # 200+ tests (3 ignorados: E2E manuales con API real)
cargo clippy --all-targets   # debe quedar a 0 warnings
cargo fmt --check # formato verificado
```

Los tests no tocan tu DB real: usan `ARQHIA_TEST_DIR` en temporales
(`db::test_guard`). Los E2E con API real están ignorados por defecto:

```sh
cargo test agent_e2e_groq -- --ignored --nocapture
```

## Documentación

- `CONTEXT/PROJECT.md` — visión, stack, arquitectura y versiones.
- `CONTEXT/ROADMAP.md` — tabla de versiones v0.1 → v1.2.
- `CONTEXT/VERSIONS/` — spec detallada de cada versión.
- `CONTEXT/POLICIES.md` — uso, privacidad, propiedad, licencias, git.
- `ARQHIA_P/docs/` — arquitectura, config, seguridad y ADRs del código.

Estado actual: ver `HECHO.md` (journal) y `CONTEXT/VERSIONS.md` (índice).
