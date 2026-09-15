# ARQHIA — POLICIES.md

Políticas de uso, privacidad, propiedad del código y licencias del STACK.

SSOT para estos temas. Referenciado desde `PROJECT.md` y los `SPECS.md` generados.
Idioma: español (docs de producto). Revisar antes de v1.0 con criterio legal real
(este documento es política de producto, no asesoría jurídica).

## 1. Uso aceptable del agente

*   El agente actúa **solo dentro del workspace asignado** más las rutas extra
    explícitas que el usuario autorice (`extra_paths`). Nunca toca el resto del disco.
*   Acciones peligrosas (escritura, consola, red, instalación, rutas externas)
    están **desactivadas por defecto** y requieren aprobación por lote, con
    descripción de qué se va a hacer. El Log registra cada acción ejecutada.
*   Prohibido usar ARQHIA para malware, intrusión, spam, evasión de controles
    de terceros o cualquier uso ilegal. El usuario responde por lo que ordena.
*   El planner (`Plan`) **nunca ejecuta herramientas**: solo lectura y plan.
    La ejecución vive en `Work` bajo permisos. Desde v0.9.1 los roles son
    5 explícitos (Orquestador > Analista > Planner > Workers > Auditor >
    loop con re-análisis); el Planner solo Net+Read (consulta docs externos
    con permiso Net + lee rutas extra con permiso Read) y nunca Write/Bash
    por construcción y por gate de rol.

## 2. Privacidad: qué viaja al proveedor LLM

Al chatear se envía al provider activo (OpenAI, Anthropic, OpenRouter, LM Studio):

*   el historial de la conversación activa,
*   el system prompt de ARQHIA,
*   en modo agente: fragmentos del workspace necesarios para la tarea
    (contenido de archivos leídos, `context_block`, `SPECS.md`, `PLAN`/`TEMP`).

No hay telemetría. No hay cuentas ni tracking. La DB (`arqhia.db`), la config
(`config.toml`) y la papelera viven en el equipo del usuario.

Aviso obligatorio en UI (v0.7): al asignar el primer workspace, mostrar que el
código del proyecto se enviará al proveedor configurado. Los paths que el
usuario marque como sensibles no se adjuntan al contexto LLM (lista en Config).

## 3. Propiedad del código

*   El código del usuario (workspace, `Project/`, `SPECS.md`, `PLAN`) **pertenece al usuario**.
*   El código generado por el agente a petición del usuario **pertenece al
    usuario** desde su creación, bajo la licencia que el usuario elija para su
    proyecto.
*   ARQHIA no reclama derechos sobre proyectos, snippets ni especificaciones.

## 4. STACK: licencias y consentimiento (desde v0.9)

Cada `stack_item` guarda `author + license + source`. Licencias admitidas al
compartir: `MIT`, `Apache-2.0` o `Propio/Uso interno` (visible solo local,
nunca sube a la nube).

Tres interruptores independientes, **todos OFF por defecto**:

1.  `use_stack` — usar código del STACK en mis tareas.
2.  `share_local` — guardar mi código en el STACK local.
3.  `share_cloud` — subir mi código a la nube (requiere sesión v1.0).

Sin consentimiento explícito no hay subida. El rating y las opiniones son
metadatos del item, no del autor.

## 5. Sesión e identidad

*   **v0.9:** identidad local (nombre + email) solo para firmar metadatos del
    STACK. Sin contraseñas, sin servidor.
*   **v1.0:** autenticación real contra el backend nube (token en Config→Nube).
    El token nunca se escribe en logs ni en `SPECS`/`TEMP`.

## 6. Licencias Pro (desde v1.1)

*   El STACK es gratis e ilimitado en todos los tiers. Pro solo desbloquea
    potencia: workers paralelos, sandboxes, agentes y flujos custom.
*   La licencia (`email + tier + exp + firma ed25519`) vive en
    `~/.config/arqhia/license.toml` con permisos 600. Se valida offline;
    no viaja código al validar.
*   Trial de 30 días sin tarjeta, 1 por email/dispositivo. Al expirar hay
    7 días de gracia en Free; los proyectos y el STACK nunca se bloquean.
*   La compra se hace en proveedor externo (Lemon Squeezy / Paddle / Gumroad);
    ARQHIA no opera ni almacena datos de pago.

## 7. Instalación y datos

*   El instalador y el **updater** nunca borran `arqhia.db`, `config.toml` ni
    workspaces. El updater (v1.0) solo **notifica** y enlaza a la descarga; no
    reemplaza binarios en caliente en esta fase.
*   Cambios de esquema SQLite son migrados con `ALTER TABLE` idempotente;
    entre versiones con instalador se documenta la migración en el CHANGELOG.
*   Exportar proyecto = copiar su carpeta (el workspace es autocontenido:
    `Project/` + `CONTEXT/` + `ToDo.md`).
*   **Consulta de versión:** el updater hace una petición anónima a
    `updates/latest.json` en la web. No envía datos del usuario ni del código.

## 8. Git y control de versiones (v0.7.2)

*   Git es **local por defecto**: cada workspace se inicializa como repo y el
    agente trabaja en la rama `ARQHIA`; la rama base (`main`/`master`) está
    protegida y nunca recibe commits del agente.
*   **Autonomía explícita** (`ReadOnly` / `CommitLocal` / `CommitAndPush`). Con
    `CommitLocal` el agente puede commitear en local; **push a GitHub está OFF
    por defecto y requiere aprobación** (categoría `GitPush`).
*   Comandos destructivos (`push --force`, `reset --hard`, `clean`, `rebase`,
    `config`, `remote add/remove`) están **bloqueados** para el agente.
*   El auto-commit solo ocurre si la puerta de calidad pasa (`cargo check` +
    `test` + `clippy`) y si el árbol de trabajo estaba limpio al iniciar el
    turno: no se commitean cambios manuales ajenos ni código roto.
*   El usuario controla rama, remoto, rama de push y autor de commits desde
    Configuración → Git. Empujar a GitHub envía código a un tercero: es
    responsabilidad del usuario revisar qué se sube.
