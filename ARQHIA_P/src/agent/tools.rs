use std::path::{Path, PathBuf};

use crate::config::{GitAutonomy, GitConfig};

/// Política de ejecución de tools (v0.7 Track B + v0.7.1 ignorados + v0.7.2 git).
/// La construye el handler desde config + aprobación del lote.
#[derive(Debug, Clone)]
pub struct ExecPolicy {
    /// Timeout de bash en segundos.
    pub timeout_s: u64,
    /// Tope de lectura por archivo en chars.
    pub max_read_chars: usize,
    /// true si auto_install o el lote fue aprobado (cubre installs).
    pub allow_install: bool,
    /// Dominios permitidos para fetch_url (comparación exacta o subdominio).
    pub net_domains: Vec<String>,
    /// true si el lote fue aprobado (cubre dominios no listados).
    pub net_approved: bool,
    /// Patrones ignorados en búsqueda/listado (v0.7.1). Vacío = defaults.
    pub ignores: Vec<String>,
    /// Autonomía git del agente (v0.7.2).
    pub git_autonomy: GitAutonomy,
    /// Push habilitado explícitamente (v0.7.2).
    pub git_push_enabled: bool,
    /// El lote git fue aprobado por el usuario (salta los gates de política).
    pub git_approved: bool,
    /// Ramas intocables (main/master por defecto).
    pub git_protected: Vec<String>,
    /// Servidores MCP por nombre (v0.9.3): para enrutar mcp__* y decidir Install/Net.
    pub mcp_servers: std::collections::HashMap<String, crate::config::McpServerConfig>,
}

impl Default for ExecPolicy {
    fn default() -> Self {
        Self {
            timeout_s: 30,
            max_read_chars: 8000,
            allow_install: false,
            net_domains: Vec::new(),
            net_approved: false,
            ignores: default_ignores(),
            git_autonomy: GitAutonomy::CommitLocal,
            git_push_enabled: false,
            git_approved: false,
            git_protected: vec!["main".to_string(), "master".to_string()],
            mcp_servers: std::collections::HashMap::new(),
        }
    }
}

/// Ignorados por defecto (v0.7.1): build, control de versiones,
/// dependencias y lockfiles. Sin confirmaciones.
/// Fuente única en `config::default_ignores` (los valores deben coincidir).
pub fn default_ignores() -> Vec<String> {
    crate::config::default_ignores()
}

/// Patrones efectivos: los configurados o los defaults si vacío.
pub fn effective_ignores(configured: &[String]) -> Vec<String> {
    if configured.is_empty() {
        default_ignores()
    } else {
        configured.to_vec()
    }
}

/// true si la ruta relativa cae en un patrón de ignorado.
/// Soporta `dir/` (ese directorio en cualquier nivel) y `*.ext`/`*.lock`.
pub fn is_ignored(rel: &str, ignores: &[String]) -> bool {
    let rel = rel.replace('\\', "/");
    let pats = if ignores.is_empty() {
        default_ignores()
    } else {
        ignores.to_vec()
    };
    for pat in &pats {
        let p = pat.trim().trim_start_matches("./");
        if p.is_empty() {
            continue;
        }
        if let Some(dir) = p.strip_suffix('/') {
            let d = dir.trim_start_matches('/');
            if rel == d || rel.starts_with(&format!("{d}/")) || rel.contains(&format!("/{d}/")) {
                return true;
            }
        } else if let Some(suf) = p.strip_prefix("*.") {
            // `*.lock` ignora cualquier archivo terminado en `.lock`.
            if rel.ends_with(&format!(".{suf}")) {
                return true;
            }
        } else if rel == p || rel.ends_with(&format!("/{p}")) {
            return true;
        }
    }
    false
}

/// Política efectiva desde config + si el lote pasó por aprobación.
#[allow(clippy::too_many_arguments)]
pub fn policy_for(
    auto_install: bool,
    batch_approved: bool,
    timeout_s: u64,
    max_read_chars: usize,
    net_domains: &[String],
    ignores: &[String],
    git: &GitConfig,
) -> ExecPolicy {
    ExecPolicy {
        timeout_s,
        max_read_chars,
        allow_install: auto_install || batch_approved,
        net_domains: net_domains.to_vec(),
        net_approved: batch_approved,
        ignores: effective_ignores(ignores),
        git_autonomy: git.autonomy,
        git_push_enabled: git.push_enabled,
        git_approved: batch_approved,
        git_protected: git.protected.clone(),
        mcp_servers: std::collections::HashMap::new(),
    }
}

/// Política efectiva con servidores MCP (v0.9.3).
#[allow(clippy::too_many_arguments)]
pub fn policy_for_with_mcp(
    auto_install: bool,
    batch_approved: bool,
    timeout_s: u64,
    max_read_chars: usize,
    net_domains: &[String],
    ignores: &[String],
    git: &GitConfig,
    mcp_servers: &std::collections::HashMap<String, crate::config::McpServerConfig>,
) -> ExecPolicy {
    let mut p = policy_for(
        auto_install,
        batch_approved,
        timeout_s,
        max_read_chars,
        net_domains,
        ignores,
        git,
    );
    p.mcp_servers = mcp_servers.clone();
    p
}

/// Resuelve `target` (relativo al workspace o absoluto) y verifica que quede
/// dentro del workspace canonicalizado. Nunca hace unwrap.
/// Normaliza léxicamente (sin tocar disco): resuelve `.` y `..`.
fn lexical_normalize(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        PathBuf::from("/")
    } else {
        out
    }
}

fn within_roots(canon: &Path, ws_canon: &Path, extra_canons: &[PathBuf]) -> bool {
    if canon.starts_with(ws_canon) {
        return true;
    }
    extra_canons.iter().any(|b| canon.starts_with(b))
}

fn resolve(workspace: &Path, extra: &[PathBuf], target: &str) -> Result<PathBuf, String> {
    let t = target.trim();
    if t.is_empty() {
        return Err("Ruta vacía".to_string());
    }
    let ws_canon = workspace
        .canonicalize()
        .map_err(|e| format!("Workspace inválido: {e}"))?;
    let extra_canons: Vec<PathBuf> = extra.iter().filter_map(|e| e.canonicalize().ok()).collect();
    let joined = if Path::new(t).is_absolute() {
        PathBuf::from(t)
    } else {
        ws_canon.join(t)
    };
    // Chequeo léxico: impide `../` y absolutos fuera del workspace,
    // pero permite archivos/carpetas aún no creados (write los crea).
    let normalized = lexical_normalize(&joined);
    let lex_ok =
        normalized.starts_with(&ws_canon) || extra_canons.iter().any(|b| normalized.starts_with(b));
    if !lex_ok {
        return Err(format!("⛔ Fuera del workspace: {t}"));
    }
    // Anti-symlink (v0.9.4): si el destino existe, su canonical debe seguir
    // dentro. Si aún no existe, se verifica el ancestro existente más
    // cercano (cubre dirs symlinkeados + symlinks colgantes).
    let mut probe = normalized.clone();
    loop {
        match std::fs::symlink_metadata(&probe) {
            Ok(md) => {
                if md.file_type().is_symlink() {
                    match probe.canonicalize() {
                        Ok(canon) => {
                            if !within_roots(&canon, &ws_canon, &extra_canons) {
                                return Err(format!("⛔ Fuera del workspace (symlink): {t}"));
                            }
                        }
                        Err(_) => {
                            return Err(format!("⛔ Fuera del workspace (symlink): {t}"));
                        }
                    }
                } else if let Ok(canon) = probe.canonicalize()
                    && !within_roots(&canon, &ws_canon, &extra_canons)
                {
                    return Err(format!("⛔ Fuera del workspace (symlink): {t}"));
                }
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let Some(parent) = probe.parent().map(|p| p.to_path_buf()) else {
                    break;
                };
                if parent.as_os_str().is_empty() {
                    break;
                }
                probe = parent;
                continue;
            }
            Err(_) => break,
        }
    }
    // El ancestro existente debe seguir dentro (dir padre symlinkeado fuera).
    if let Ok(canon) = probe.canonicalize()
        && !within_roots(&canon, &ws_canon, &extra_canons)
    {
        return Err(format!("⛔ Fuera del workspace (symlink): {t}"));
    }
    Ok(normalized)
}

fn truncate(s: &str, max: usize) -> String {
    // Por chars, nunca por bytes (cortar UTF-8 por bytes = panic).
    let n = s.chars().count();
    if n <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}…[truncado {} chars]", n - max)
    }
}

/// Lee un archivo por páginas de líneas (v0.7.1).
/// `offset` 1-based (def 1), `limit` líneas por página (def 200, tope 1000).
/// Responde `líneas X–Y de Z total` + líneas numeradas, para que el modelo
/// pida más páginas en vez de tragar archivos enteros.
/// `max_chars` sigue truncando el cuerpo como red de seguridad.
pub async fn read_file(
    workspace: &Path,
    extra: &[PathBuf],
    target: &str,
    offset: usize,
    limit: usize,
    max_chars: usize,
) -> Result<String, String> {
    let path = resolve(workspace, extra, target)?;
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("No se pudo leer {}: {e}", path.display()))?;
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    if total == 0 {
        return Ok(format!("{}: (vacío, 0 líneas)", target.trim()));
    }
    let start = offset.max(1) - 1;
    if start >= total {
        return Err(format!(
            "offset {offset} más allá del final ({total} líneas en {})",
            target.trim()
        ));
    }
    let per_page = limit.clamp(1, 1000);
    let end = (start + per_page).min(total);
    let mut out = format!(
        "líneas {}–{} de {} total ({})\n",
        start + 1,
        end,
        total,
        target.trim()
    );
    for (i, line) in lines[start..end].iter().enumerate() {
        let clean: String = line.chars().take(300).collect();
        out.push_str(&format!("{}| {clean}\n", start + i + 1));
    }
    if end < total {
        out.push_str(&format!(
            "…siguen {} líneas (pide offset {})",
            total - end,
            end + 1
        ));
    }
    Ok(truncate(&out, max_chars.max(1024)))
}

pub async fn write_file(
    workspace: &Path,
    extra: &[PathBuf],
    target: &str,
    content: &str,
) -> Result<String, String> {
    let path = resolve(workspace, extra, target)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("No se pudo crear dirs: {e}"))?;
    }
    tokio::fs::write(&path, content)
        .await
        .map_err(|e| format!("No se pudo escribir: {e}"))?;
    Ok(format!(
        "✅ write {} ({} bytes)",
        rel(workspace, &path),
        content.len()
    ))
}

pub async fn edit_file(
    workspace: &Path,
    extra: &[PathBuf],
    target: &str,
    old: &str,
    new: &str,
) -> Result<String, String> {
    let path = resolve(workspace, extra, target)?;
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("No se pudo leer {}: {e}", path.display()))?;
    let matches = content.matches(old).count();
    if matches == 0 {
        return Err("edit: texto `old` no encontrado (0 coincidencias)".to_string());
    }
    if matches > 1 {
        return Err(format!(
            "edit: `old` ambiguo ({matches} coincidencias, se esperaba 1)"
        ));
    }
    let updated = content.replacen(old, new, 1);
    tokio::fs::write(&path, updated)
        .await
        .map_err(|e| format!("No se pudo escribir: {e}"))?;
    Ok(format!("✅ edit {}", rel(workspace, &path)))
}

pub async fn delete_file(
    workspace: &Path,
    extra: &[PathBuf],
    target: &str,
) -> Result<String, String> {
    let path = resolve(workspace, extra, target)?;
    if !path.exists() {
        return Err(format!("No existe: {target}"));
    }
    if path.is_dir() {
        return Err("delete solo borra archivos, no carpetas".to_string());
    }
    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| format!("No se pudo borrar: {e}"))?;
    Ok(format!("✅ delete {}", rel(workspace, &path)))
}

pub async fn list_dir(
    workspace: &Path,
    extra: &[PathBuf],
    target: &str,
    ignores: &[String],
) -> Result<String, String> {
    let dir = if target.trim().is_empty() || target.trim() == "." {
        workspace
            .canonicalize()
            .map_err(|e| format!("Workspace inválido: {e}"))?
    } else {
        resolve(workspace, extra, target)?
    };
    if !dir.is_dir() {
        return Err(format!("No es carpeta: {target}"));
    }
    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .map_err(|e| format!("No se pudo listar: {e}"))?;
    let mut names: Vec<String> = Vec::new();
    while let Ok(Some(e)) = entries.next_entry().await {
        let n = e.file_name().to_string_lossy().to_string();
        let suffixed = if e.path().is_dir() {
            format!("{n}/")
        } else {
            n.clone()
        };
        // v0.7.1: filtra ignorados también en el listado directo.
        if is_ignored(&suffixed, ignores) || is_ignored(&n, ignores) {
            continue;
        }
        names.push(if e.path().is_dir() {
            format!("{n}/")
        } else {
            n
        });
        if names.len() >= 100 {
            break;
        }
    }
    names.sort();
    if names.is_empty() {
        Ok("(vacío)".to_string())
    } else {
        Ok(names.join("\n"))
    }
}

/// Tokeniza sin shell (v0.9.4): respeta comillas simples/dobles, sin
/// expansiones. `sh -c` ya no se usa: esto evita `;`, `&&`, `$()` etc.
pub fn split_argv(cmd: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut in_token = false;
    let mut chars = cmd.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else if c == '\\' && q == '"' {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            } else {
                cur.push(c);
            }
            in_token = true;
            continue;
        }
        match c {
            '\'' | '"' => {
                quote = Some(c);
                in_token = true;
            }
            c if c.is_whitespace() => {
                if in_token {
                    out.push(std::mem::take(&mut cur));
                    in_token = false;
                }
            }
            _ => {
                cur.push(c);
                in_token = true;
            }
        }
    }
    if in_token || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// true si hay metacaracteres de shell fuera de comillas (`;|&$`...).
/// Con `exec` directo serían inofensivos, pero se deniegan por allowlist
/// exacta: `cargo test; curl` nunca debe ni intentarse.
fn contains_shell_metachars(cmd: &str) -> bool {
    let mut quote: Option<char> = None;
    for c in cmd.chars() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' => quote = Some(c),
            ';' | '|' | '`' | '>' | '<' | '\n' => return true,
            '&' | '$' | '!' | '*' | '?' | '~' | '(' | ')' | '{' | '}' => return true,
            _ => {}
        }
    }
    false
}

/// Comandos permitidos (v0.9.4: `exec` directo sin shell, argv[0] exacto).
/// `pub` para testear la política sin ejecutar nada.
pub fn is_allowed(cmd: &str) -> bool {
    let c = cmd.trim();
    if c.is_empty() || contains_shell_metachars(c) {
        return false;
    }
    // Instaladores y sudo pasan al gate de permiso (executor): sin
    // auto_install o aprobación del lote, se deniegan allí.
    if INSTALL_PREFIXES
        .iter()
        .any(|p| c == *p || c.starts_with(&format!("{p} ")))
        || c.starts_with("sudo ")
    {
        return true;
    }
    let argv = split_argv(c);
    if argv.is_empty() {
        return false;
    }
    match argv[0].as_str() {
        "ls" | "cat" | "echo" | "pwd" => true,
        "rustc" => argv.get(1).is_some_and(|a| a == "--version" || a == "-V"),
        "cargo" => matches!(
            argv.get(1).map(|s| s.as_str()),
            Some("--version" | "check" | "build" | "test" | "run" | "clippy" | "fmt")
        ),
        // Git: el allowlist deja pasar; la política por comando (classify_git)
        // decide en el executor qué se ejecuta y qué pide permiso.
        "git" => true,
        _ => false,
    }
}

fn allowed_list() -> &'static str {
    "ls, cat, echo, pwd, cargo --version, rustc --version, cargo check, cargo build, cargo test, cargo run, cargo clippy, cargo fmt, git (política por subcomando) (+instaladores solo con permiso Install)"
}

/// Prefijos que instalan software (v0.7 Track B): categoría Install,
/// siempre piden permiso salvo auto_install o lote aprobado.
const INSTALL_PREFIXES: [&str; 10] = [
    "cargo install",
    "pip install",
    "pip3 install",
    "npm install -g",
    "npm i -g",
    "apt install",
    "apt-get install",
    "dnf install",
    "pacman -S",
    "brew install",
];

/// true si el comando instala software (incluye cualquier `sudo ...`,
/// conservador por seguridad).
pub fn is_install_cmd(cmd: &str) -> bool {
    let c = cmd.trim();
    INSTALL_PREFIXES
        .iter()
        .any(|p| c == *p || c.starts_with(&format!("{p} ")))
        || c.starts_with("sudo ")
}

/// Clasificación de un comando `git ...` (v0.7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitKind {
    /// status, diff, log, show, branch, rev-parse, remote (consulta).
    Read,
    /// init, add, commit, stash, reset (no --hard).
    Write,
    /// push, fetch, pull.
    Net,
    /// Destructivos/desconocidos: nunca se ejecutan.
    Blocked,
}

/// Clasifica un comando `git`. `None` si no empieza por `git`.
/// Conservador: subcomandos no reconocidos o flags globales raros se
/// consideran bloqueados.
pub fn classify_git(cmd: &str) -> Option<GitKind> {
    let c = cmd.trim();
    let rest = c.strip_prefix("git")?.trim_start();
    if rest.is_empty() {
        return Some(GitKind::Read);
    }
    let lower = rest.to_lowercase();
    let mut words = lower.split_whitespace();
    let first = words.next().unwrap_or("");
    // Flags globales: solo --version/--help son lectura. Cualquier otro
    // (p.ej. -C para salir del workspace) se bloquea.
    if first.starts_with('-') {
        return Some(if matches!(first, "--version" | "--help" | "-h" | "-v") {
            GitKind::Read
        } else {
            GitKind::Blocked
        });
    }
    let second = words.next().unwrap_or("");
    match first {
        "status" | "diff" | "log" | "show" | "branch" | "rev-parse" => Some(GitKind::Read),
        "remote" => {
            if matches!(second, "add" | "remove" | "rm" | "set-url" | "rename") {
                Some(GitKind::Blocked)
            } else {
                Some(GitKind::Read)
            }
        }
        "init" | "add" | "commit" | "stash" => Some(GitKind::Write),
        "push" => {
            if lower.contains("--force") || lower.split_whitespace().any(|a| a == "-f") {
                Some(GitKind::Blocked)
            } else {
                Some(GitKind::Net)
            }
        }
        "fetch" | "pull" => Some(GitKind::Net),
        "reset" => {
            if lower.contains("--hard") {
                Some(GitKind::Blocked)
            } else {
                Some(GitKind::Write)
            }
        }
        // Destructivos y cambios de rama: bloqueados siempre.
        _ => Some(GitKind::Blocked),
    }
}

/// Clasificación git a partir de los args de una call `bash`.
pub fn git_call_kind(args: &serde_json::Value) -> Option<GitKind> {
    let cmd = args.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
    classify_git(cmd)
}

/// true si el comando es `git commit` (para la guarda de rama protegida).
fn is_git_commit(cmd: &str) -> bool {
    cmd.trim()
        .strip_prefix("git")
        .map(|r| {
            let r = r.trim_start();
            r == "commit" || r.starts_with("commit ")
        })
        .unwrap_or(false)
}

/// Parsea una URL http(s) y devuelve `(host, puerto)` con el host en
/// minúsculas, SIN credenciales (`user:pass@`) ni puerto, o `None` si no es
/// http(s) o no tiene host. Es la base del anti-SSRF: separar bien el host
/// evita el bypass `http://evil.com@127.0.0.1/` (v0.9.5).
fn parse_http_url(url: &str) -> Option<(String, u16)> {
    let u = url.trim();
    let (rest, default_port) = if let Some(r) = u.strip_prefix("https://") {
        (r, 443u16)
    } else {
        let r = u.strip_prefix("http://")?;
        (r, 80u16)
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // Quita userinfo: `user:pass@host` -> `host`.
    let hostport = authority.rsplit('@').next().unwrap_or(authority);
    if hostport.is_empty() {
        return None;
    }
    let (host, port) = if let Some(after_bracket) = hostport.strip_prefix('[') {
        // IPv6: `[::1]:8000` -> host `::1`, puerto opcional.
        let close = after_bracket.find(']')?;
        let host = &after_bracket[..close];
        let port = after_bracket[close + 1..]
            .strip_prefix(':')
            .and_then(|p| p.parse().ok())
            .unwrap_or(default_port);
        (host.to_string(), port)
    } else if hostport.matches(':').count() == 1 {
        let mut parts = hostport.splitn(2, ':');
        let h = parts.next().unwrap_or("");
        let p = parts
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(default_port);
        (h.to_string(), p)
    } else {
        (hostport.to_string(), default_port)
    };
    // Minúsculas y sin punto final (`127.0.0.1.` no debe colarse).
    let host = host.trim().trim_matches('.').to_lowercase();
    if host.is_empty() {
        return None;
    }
    Some((host, port))
}

/// Host de una URL http(s) en minúsculas, o "" si inválida.
fn url_host(url: &str) -> String {
    parse_http_url(url).map(|(h, _)| h).unwrap_or_default()
}

/// true si la IP resuelta es interna/no enrutable (anti-SSRF v0.9.5):
/// loopback, privadas, link-local, CGNAT, multicast, sin especificar,
/// documentación y equivalencias IPv6 (incl. IPv4-mapped).
fn is_blocked_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_multicast()
                || o[0] == 0
                // 100.64.0.0/10 (CGNAT).
                || (o[0] == 100 && (o[1] & 0xc0) == 64)
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_blocked_ip(IpAddr::V4(mapped));
            }
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || v6.is_multicast()
        }
    }
}

/// Resuelve el host y comprueba que NINGUNA IP de destino sea interna.
/// Si la resolución falla, devuelve false (reqwest reportará el error real).
/// Corre en `spawn_blocking`: `getaddrinfo` bloquea.
async fn host_resolves_to_blocked(host: &str, port: u16) -> bool {
    let host = host.to_string();
    tokio::task::spawn_blocking(move || {
        use std::net::ToSocketAddrs;
        match (host.as_str(), port).to_socket_addrs() {
            Ok(mut addrs) => addrs.any(|a| is_blocked_ip(a.ip())),
            Err(_) => false,
        }
    })
    .await
    .unwrap_or(false)
}

/// true si el host está en la allowlist (exacto o subdominio).
pub fn url_domain_listed(url: &str, domains: &[String]) -> bool {
    let host = url_host(url);
    if host.is_empty() {
        return false;
    }
    domains.iter().any(|d| {
        let dd = d.trim().to_lowercase();
        !dd.is_empty() && (host == dd || host.ends_with(&format!(".{dd}")))
    })
}

/// true si el host (o una IP literal) apunta a un destino interno (SSRF).
/// Normaliza userinfo, corchetes IPv6 y puerto; bloquea `localhost`,
/// nombres de metadatos y cualquier IP interna vía `is_blocked_ip`.
/// Para nombres de dominio la validación real ocurre tras resolver DNS
/// (`host_resolves_to_blocked`).
pub fn is_ssrf_host(host: &str) -> bool {
    let h = host.trim().to_lowercase();
    // Sin credenciales ni punto final.
    let h = h.rsplit('@').next().unwrap_or(&h);
    let bare = if let Some(stripped) = h.strip_prefix('[') {
        stripped.split(']').next().unwrap_or("").trim()
    } else if h.matches(':').count() > 1 {
        h
    } else {
        h.split(':').next().unwrap_or("").trim()
    };
    let bare = bare
        .trim_matches(|c| c == '[' || c == ']')
        .trim_matches('.');
    if bare.is_empty() {
        return true;
    }
    if bare == "localhost"
        || bare.ends_with(".localhost")
        || bare == "metadata.google.internal"
        || bare == "metadata"
    {
        return true;
    }
    if let Ok(ip) = bare.parse::<std::net::IpAddr>() {
        return is_blocked_ip(ip);
    }
    false
}

/// GET http(s) con timeout 20s y tope 64k chars (v0.7 Track B).
/// Dominios: listados siempre; no listados solo con lote aprobado.
/// Anti-SSRF (v0.9.4): bloquea loopback/metadatos y sigue máx 3 redirects
/// re-validando cada salto (sin petición al destino bloqueado).
pub async fn fetch_url(url: &str, policy: &ExecPolicy) -> Result<String, String> {
    let mut current = url.trim().to_string();
    for _ in 0..4 {
        let Some((host, port)) = parse_http_url(&current) else {
            return Err("URL inválida (solo http/https)".to_string());
        };
        if is_ssrf_host(&host) {
            return Err(format!("⛔ Destino interno bloqueado (SSRF): {host}"));
        }
        // Anti-SSRF real: valida también las IPs resueltas por DNS (cubre
        // dominios que apuntan a red interna y ataques de rebinding básicos).
        if host_resolves_to_blocked(&host, port).await {
            return Err(format!(
                "⛔ Destino interno bloqueado tras resolver DNS (SSRF): {host}"
            ));
        }
        if !url_domain_listed(&current, &policy.net_domains) && !policy.net_approved {
            return Err(format!("⛔ Dominio no autorizado: {host}"));
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("Error HTTP: {e}"))?;
        let resp = client
            .get(current.clone())
            .send()
            .await
            .map_err(|e| format!("Error de red: {e}"))?;
        if resp.status().is_redirection() {
            let Some(next) = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
            else {
                return Err(format!("Redirect sin Location: {}", resp.status()));
            };
            // Relativo respecto al actual; absoluto tal cual.
            if next.starts_with("http://") || next.starts_with("https://") {
                current = next.to_string();
            } else if let Some(base_end) = current
                .find("://")
                .and_then(|i| current[i + 3..].find('/').map(|j| i + 3 + j))
            {
                current = format!("{}{}", &current[..base_end], next);
            } else {
                current = format!("{current}{next}");
            }
            continue;
        }
        if !resp.status().is_success() {
            return Err(format!("HTTP {}: {}", resp.status(), current));
        }
        let text = resp.text().await.unwrap_or_default();
        if text.trim().is_empty() {
            return Ok("(respuesta vacía)".to_string());
        }
        return Ok(truncate(&text, 64_000));
    }
    Err("Demasiados redirects (máx 3)".to_string())
}

pub async fn bash(
    workspace: &Path,
    extra: &[PathBuf],
    cmd: &str,
    policy: &ExecPolicy,
) -> Result<String, String> {
    let _ = extra;
    if !is_allowed(cmd) {
        return Err(format!(
            "⛔ Comando no permitido: `{}`. Permitidos: {}",
            cmd.trim(),
            allowed_list()
        ));
    }
    // Instaladores: solo con auto_install o aprobación del lote.
    if is_install_cmd(cmd) && !policy.allow_install {
        return Err(
            "⛔ Instalación no aprobada: actívala en Permisos o aprueba la acción.".to_string(),
        );
    }
    // Política git por comando (v0.7.2): lo destructivo se bloquea siempre;
    // el resto respeta autonomía/push salvo que el lote esté aprobado.
    if let Some(kind) = classify_git(cmd) {
        if matches!(kind, GitKind::Blocked) {
            return Err(format!(
                "⛔ comando git bloqueado por seguridad: `{}`",
                cmd.trim()
            ));
        }
        if matches!(kind, GitKind::Write)
            && is_git_commit(cmd)
            && let Some(br) = crate::git::current_branch_async(workspace).await
            && policy.git_protected.iter().any(|p| p == &br)
        {
            return Err(format!(
                "⛔ commit sobre rama protegida `{br}` (usa la rama de trabajo)"
            ));
        }
        if !policy.git_approved {
            let allowed = match kind {
                GitKind::Read => true,
                GitKind::Write => policy.git_autonomy >= GitAutonomy::CommitLocal,
                GitKind::Net => {
                    policy.git_push_enabled && policy.git_autonomy == GitAutonomy::CommitAndPush
                }
                GitKind::Blocked => false,
            };
            if !allowed {
                return Err(format!(
                    "⛔ acción git no permitida por la política: `{}`. Apruébala o ajústala en Config → Git.",
                    cmd.trim()
                ));
            }
        }
    }
    let ws = workspace
        .canonicalize()
        .map_err(|e| format!("Workspace inválido: {e}"))?;
    // v0.9.4: `exec` directo sin shell. `split_argv` ya validó que no hay
    // metacaracteres; aquí argv[0] es el binario exacto.
    let argv = split_argv(cmd.trim());
    if argv.is_empty() {
        return Err("Comando vacío".to_string());
    }
    let child = tokio::process::Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(&ws)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("No se pudo ejecutar: {e}"))?;
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(policy.timeout_s.max(1)),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| format!("⏱ bash excedió {}s", policy.timeout_s.max(1)))?
    .map_err(|e| format!("Falló ejecución: {e}"))?;
    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&out.stdout));
    if !out.stderr.is_empty() {
        combined.push_str("\n[stderr]\n");
        combined.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    if !out.status.success() {
        combined.push_str(&format!("\n[exit {}]", out.status));
    }
    if combined.trim().is_empty() {
        Ok("(sin salida)".to_string())
    } else {
        Ok(truncate(&combined, 4000))
    }
}

fn rel(workspace: &Path, path: &Path) -> String {
    let ws = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    path.strip_prefix(&ws)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

/// Categoría de permiso de cada tool (v0.6 + v0.7 Track B + v0.7.2 git).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCat {
    Read,
    Write,
    Bash,
    /// Red (fetch_url).
    Net,
    /// Instaladores (bash con install o sudo).
    Install,
    /// Comandos git locales (lectura o escritura de repo).
    Git,
    /// Git que toca la red (push/fetch/pull).
    GitPush,
}

pub fn category(name: &str) -> ToolCat {
    match name {
        "write_file" | "edit_file" | "delete_file" => ToolCat::Write,
        "fetch_url" => ToolCat::Net,
        "bash" => ToolCat::Bash,
        _ => ToolCat::Read,
    }
}

/// Categoría real de una llamada (bash puede ser Install o git según args).
pub fn category_of_call(name: &str, args: &serde_json::Value) -> ToolCat {
    // MCP v0.9.3: mcp__srv__tool -> Net salvo auto=true (Read) o Install si command es npx.
    if crate::mcp::is_mcp_tool(name) {
        return ToolCat::Net;
    }
    if name == "bash" {
        let cmd = args.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(kind) = classify_git(cmd) {
            return match kind {
                GitKind::Net => ToolCat::GitPush,
                _ => ToolCat::Git,
            };
        }
        if is_install_cmd(cmd) {
            return ToolCat::Install;
        }
        return ToolCat::Bash;
    }
    category(name)
}

/// Categoría de una llamada MCP conociendo la config del servidor (v0.9.3).
/// `auto=true` -> Read (automático), `command=npx` -> Install, resto -> Net.
pub fn category_of_mcp_call(
    name: &str,
    servers: &std::collections::HashMap<String, crate::config::McpServerConfig>,
) -> ToolCat {
    let Some((srv, _)) = crate::mcp::parse_mcp_tool(name) else {
        return ToolCat::Net;
    };
    if let Some(cfg) = servers.get(&srv) {
        if cfg.is_install() {
            return ToolCat::Install;
        }
        if cfg.auto {
            return ToolCat::Read;
        }
    }
    ToolCat::Net
}

/// Bloque de coincidencia con contexto (v0.7.1): 3 líneas antes/después,
/// nunca el archivo completo.
#[derive(Debug, Clone)]
struct MatchBlock {
    /// ¿El nombre del archivo contiene la query? (ranking primero).
    name_hit: bool,
    /// Coincidencias en el mismo archivo (ranking segundo).
    file_hits: usize,
    /// Primera línea del archivo donde aparece (desempate).
    first_line: usize,
    /// Línea única `path:línea: ctx » match « ctx`.
    text: String,
}

/// Busca `query` (subcadena, insensible a mayúsculas) en archivos de texto
/// del workspace (v0.7.1).
/// Devuelve máx 20 bloques `path:línea: ctx » match « ctx` con 3 líneas de
/// contexto, rankeados (nombre de archivo > contenido; hits del mismo
/// archivo agrupados). Ignora `target/`, `.git/`, `node_modules/`, `*.lock`
/// y binarios. Nunca devuelve archivos completos.
pub async fn search_files(
    workspace: &Path,
    extra: &[PathBuf],
    query: &str,
    dir: &str,
    ignores: &[String],
) -> Result<String, String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Err("Búsqueda vacía".to_string());
    }
    let base = if dir.trim().is_empty() || dir.trim() == "." {
        workspace
            .canonicalize()
            .map_err(|e| format!("Workspace inválido: {e}"))?
    } else {
        resolve(workspace, extra, dir)?
    };
    if !base.is_dir() {
        return Err(format!("No es carpeta: {dir}"));
    }
    let ws_canon = workspace
        .canonicalize()
        .map_err(|e| format!("Workspace inválido: {e}"))?;
    // Primera pasada: junta coincidencias por archivo.
    struct FileHits {
        rel: String,
        name_hit: bool,
        lines: Vec<String>,
        hits: Vec<usize>,
    }
    let mut files: Vec<FileHits> = Vec::new();
    let walker = walkdir::WalkDir::new(&base)
        .max_depth(8)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());
    for entry in walker {
        let path = entry.path();
        // v0.9.4 anti-symlink: un symlink que apunte fuera no se sigue.
        if let Ok(md) = std::fs::symlink_metadata(path)
            && md.file_type().is_symlink()
            && let Ok(canon) = path.canonicalize()
            && !canon.starts_with(&ws_canon)
        {
            continue;
        }
        let rel_s = path
            .strip_prefix(&ws_canon)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if is_ignored(&rel_s, ignores) {
            continue;
        }
        let Ok(meta) = std::fs::metadata(path) else {
            continue;
        };
        if meta.len() > 200_000 {
            continue;
        }
        // Binarios: read_to_string falla -> se saltan sin ruido.
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        let mut hits = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains(&q) {
                hits.push(i);
            }
            if hits.len() >= 12 {
                break; // tope por archivo: el ranking decide el resto
            }
        }
        if hits.is_empty() {
            continue;
        }
        let fname = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        files.push(FileHits {
            rel: rel_s,
            name_hit: fname.contains(&q),
            lines,
            hits,
        });
        if files.len() >= 60 {
            break; // tope de archivos escaneados con hits
        }
    }
    if files.is_empty() {
        return Ok(format!("Sin resultados para '{query}'"));
    }
    // Ranking: nombre > nº hits > primera aparición.
    files.sort_by(|a, b| {
        b.name_hit
            .cmp(&a.name_hit)
            .then(b.hits.len().cmp(&a.hits.len()))
            .then(a.hits[0].cmp(&b.hits[0]))
    });
    // Segunda pasada: bloques con contexto, máx 20 en total.
    let mut blocks: Vec<MatchBlock> = Vec::new();
    for f in &files {
        for &h in &f.hits {
            if blocks.len() >= 20 {
                break;
            }
            let before: Vec<String> = f.lines[h.saturating_sub(3)..h]
                .iter()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|l| l.chars().take(60).collect())
                .collect();
            let after: Vec<String> = f.lines[h + 1..(h + 4).min(f.lines.len())]
                .iter()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|l| l.chars().take(60).collect())
                .collect();
            let central: String = f.lines[h].trim().chars().take(120).collect();
            let mut ctx = String::new();
            if !before.is_empty() {
                ctx.push_str(&before.join(" / "));
                ctx.push_str(" » ");
            }
            ctx.push_str(&central);
            if !after.is_empty() {
                ctx.push_str(" « ");
                ctx.push_str(&after.join(" / "));
            }
            blocks.push(MatchBlock {
                name_hit: f.name_hit,
                file_hits: f.hits.len(),
                first_line: h,
                text: format!("{}:{}: {}", f.rel, h + 1, ctx),
            });
        }
        if blocks.len() >= 20 {
            break;
        }
    }
    // Orden final por bloque: nombre > hits del archivo > línea.
    blocks.sort_by(|a, b| {
        b.name_hit
            .cmp(&a.name_hit)
            .then(b.file_hits.cmp(&a.file_hits))
            .then(a.first_line.cmp(&b.first_line))
    });
    let mut out: Vec<String> = blocks.into_iter().take(20).map(|b| b.text).collect();
    if files.len() >= 60 || out.len() == 20 {
        out.push("(top 20 bloques; afina la query o pide outline de un archivo)".to_string());
    }
    if out.is_empty() {
        Ok(format!("Sin resultados para '{query}'"))
    } else {
        Ok(out.join("\n"))
    }
}

/// Firma de un archivo para `get_file_outline` (v0.7.1): funciones,
/// structs, imports… según el lenguaje por extensión. `None` = ruido.
fn outline_sig(line: &str) -> Option<String> {
    let t = line.trim();
    if t.is_empty() || t.starts_with("//") || t.starts_with('#') && t.starts_with("#!") {
        return None;
    }
    // Prefijos por lenguaje (Rust, Python, JS/TS, Go, markdown headers).
    const PREFIXES: [&str; 24] = [
        "fn ",
        "pub fn ",
        "pub async fn ",
        "async fn ",
        "struct ",
        "pub struct ",
        "enum ",
        "pub enum ",
        "trait ",
        "pub trait ",
        "impl ",
        "mod ",
        "pub mod ",
        "use ",
        "type ",
        "pub type ",
        "const ",
        "pub const ",
        "static ",
        "def ",
        "class ",
        "import ",
        "from ",
        "func ",
    ];
    // Normaliza `pub(crate)`, `pub(super)`, `export`, `async`, `pub ` líderes.
    let mut rest = t;
    for strip in [
        "pub(crate) ",
        "pub(super) ",
        "pub(in ",
        "export default ",
        "export ",
        "async ",
        "pub ",
    ] {
        if let Some(s) = rest.strip_prefix(strip) {
            rest = s.trim_start();
        }
    }
    // Headers markdown como outline.
    if rest.starts_with('#') {
        return Some(t.chars().take(100).collect());
    }
    // `const X =`, `let X = ... =>` (JS/TS top-level).
    for pref in PREFIXES {
        if rest.starts_with(pref) {
            return Some(t.chars().take(120).collect());
        }
    }
    // JS/TS: `function name(`, `X = (args) =>`, `X: (...) =>` dentro de objetos.
    if rest.starts_with("function ") || rest.contains("=>") && rest.contains('(') {
        return Some(t.chars().take(120).collect());
    }
    None
}

/// Solo firmas del archivo (v0.7.1): funciones, structs, imports…
/// Flujo esperado: outline -> search -> read paginado.
/// Máx 100 firmas `línea: firma` + total de líneas.
pub async fn get_file_outline(
    workspace: &Path,
    extra: &[PathBuf],
    target: &str,
) -> Result<String, String> {
    let path = resolve(workspace, extra, target)?;
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("No se pudo leer {}: {e}", path.display()))?;
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let mut sigs: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(sig) = outline_sig(line) {
            sigs.push(format!("{}: {sig}", i + 1));
            if sigs.len() >= 100 {
                break;
            }
        }
    }
    if sigs.is_empty() {
        return Ok(format!(
            "{}: sin firmas detectadas ({total} líneas; usa search o read)",
            target.trim()
        ));
    }
    let mut out = format!(
        "outline de {} ({} firmas, {} líneas total)\n",
        target.trim(),
        sigs.len(),
        total
    );
    out.push_str(&sigs.join("\n"));
    if sigs.len() == 100 {
        out.push_str("\n…(top 100 firmas)");
    }
    Ok(out)
}

/// Firmas de un archivo (v0.7.3) para el analista: reutiliza `outline_sig`
/// sin pasar por el guard (la ruta ya viene del workspace). Devuelve "" si
/// no se puede leer.
pub fn outline_of(path: &Path) -> String {
    let Ok(content) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let mut sigs: Vec<String> = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if let Some(sig) = outline_sig(line) {
            sigs.push(format!("{}: {sig}", i + 1));
            if sigs.len() >= 60 {
                break;
            }
        }
    }
    sigs.join("\n")
}

/// Despacha una tool por nombre con args JSON. Retorna texto para el LLM/UI.
pub async fn execute(
    workspace: &Path,
    extra: &[PathBuf],
    name: &str,
    args: &serde_json::Value,
    policy: &ExecPolicy,
) -> String {
    let res = match name {
        "read_file" => {
            let p = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            // v0.7.1 paginado: offset 1-based, limit líneas por página.
            let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
            read_file(workspace, extra, p, offset, limit, policy.max_read_chars).await
        }
        "write_file" => {
            let p = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let c = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            write_file(workspace, extra, p, c).await
        }
        "edit_file" => {
            let p = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let o = args.get("old").and_then(|v| v.as_str()).unwrap_or("");
            let n = args.get("new").and_then(|v| v.as_str()).unwrap_or("");
            edit_file(workspace, extra, p, o, n).await
        }
        "delete_file" => {
            let p = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            delete_file(workspace, extra, p).await
        }
        "list_dir" => {
            let p = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            list_dir(workspace, extra, p, &policy.ignores).await
        }
        "bash" => {
            let c = args.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
            bash(workspace, extra, c, policy).await
        }
        "search_files" => {
            let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let d = args.get("dir").and_then(|v| v.as_str()).unwrap_or(".");
            search_files(workspace, extra, q, d, &policy.ignores).await
        }
        "get_file_outline" => {
            let p = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            get_file_outline(workspace, extra, p).await
        }
        "fetch_url" => {
            let u = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
            fetch_url(u, policy).await
        }
        other if crate::mcp::is_mcp_tool(other) => execute_mcp(other, args, policy).await,
        other => Err(format!("Tool desconocida: {other}")),
    };
    match res {
        Ok(ok) => ok,
        Err(e) => format!("❌ {e}"),
    }
}

async fn execute_mcp(
    name: &str,
    args: &serde_json::Value,
    policy: &ExecPolicy,
) -> Result<String, String> {
    let Some((srv, tool)) = crate::mcp::parse_mcp_tool(name) else {
        return Err(format!("MCP tool inválida: {name}"));
    };
    let cfg = policy
        .mcp_servers
        .get(&srv)
        .ok_or_else(|| format!("Servidor MCP no configurado: {srv}"))?;
    // npx etc requiere Install
    if cfg.is_install() && !policy.allow_install {
        return Err("⛔ MCP con npx/uvx requiere permiso Install.".to_string());
    }
    // auto=false ya cubierto por categoría, pero aquí respetamos timeout
    // args es el input del tool (ya viene como objeto JSON)
    let input = if args.is_object() {
        args.clone()
    } else {
        serde_json::json!({})
    };
    // Si el método es resources/* o prompts/* -> error hasta v1.0.1
    if tool.starts_with("resources/") || tool.starts_with("prompts/") {
        return Err("resources/prompts hasta v1.0.1".to_string());
    }
    match cfg.transport {
        crate::config::McpTransport::Stdio => crate::mcp::call_tool_stdio(cfg, &tool, input).await,
        crate::config::McpTransport::Http => crate::mcp::call_tool_http(cfg, &tool, input).await,
    }
}

/// Helpers para anexar schemas MCP a los nativos (v0.9.3).
pub fn mcp_openai_schemas(
    collected: &std::collections::HashMap<String, Vec<crate::mcp::McpTool>>,
) -> serde_json::Value {
    let mut out = Vec::new();
    for (srv, tools) in collected {
        for t in tools {
            out.push(crate::mcp::tool_to_openai_schema(srv, t));
        }
    }
    serde_json::Value::Array(out)
}
pub fn mcp_anthropic_schemas(
    collected: &std::collections::HashMap<String, Vec<crate::mcp::McpTool>>,
) -> serde_json::Value {
    let mut out = Vec::new();
    for (srv, tools) in collected {
        for t in tools {
            out.push(crate::mcp::tool_to_anthropic_schema(srv, t));
        }
    }
    serde_json::Value::Array(out)
}

/// Combina schemas nativos + MCP (trunca aviso si > 20 tools totales).
pub fn openai_schemas_with_mcp(
    collected: &std::collections::HashMap<String, Vec<crate::mcp::McpTool>>,
) -> serde_json::Value {
    let mut base = openai_schemas();
    let mcp = mcp_openai_schemas(collected);
    if let (Some(arr), Some(marr)) = (base.as_array_mut(), mcp.as_array()) {
        arr.extend(marr.clone());
    }
    base
}
pub fn anthropic_schemas_with_mcp(
    collected: &std::collections::HashMap<String, Vec<crate::mcp::McpTool>>,
) -> serde_json::Value {
    let mut base = anthropic_schemas();
    let mcp = mcp_anthropic_schemas(collected);
    if let (Some(arr), Some(marr)) = (base.as_array_mut(), mcp.as_array()) {
        arr.extend(marr.clone());
    }
    base
}

/// Schemas OpenAI-compatible para function calling.
pub fn openai_schemas() -> serde_json::Value {
    serde_json::json!([
        {"type": "function", "function": {"name": "read_file", "description": "Lee un archivo del workspace POR PÁGINAS (200 líneas por defecto). Responde líneas X–Y de Z total con nº de línea. Pide más páginas con offset.", "parameters": {"type": "object", "properties": {"path": {"type": "string"}, "offset": {"type": "integer"}, "limit": {"type": "integer"}}, "required": ["path"]}}},
        {"type": "function", "function": {"name": "write_file", "description": "Crea o sobrescribe un archivo del workspace. Crea carpetas padre.", "parameters": {"type": "object", "properties": {"path": {"type": "string"}, "content": {"type": "string"}}, "required": ["path", "content"]}}},
        {"type": "function", "function": {"name": "edit_file", "description": "Reemplaza EXACTAMENTE 1 ocurrencia de `old` por `new`.", "parameters": {"type": "object", "properties": {"path": {"type": "string"}, "old": {"type": "string"}, "new": {"type": "string"}}, "required": ["path", "old", "new"]}}},
        {"type": "function", "function": {"name": "delete_file", "description": "Borra un archivo del workspace.", "parameters": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}}},
        {"type": "function", "function": {"name": "list_dir", "description": "Lista una carpeta del workspace.", "parameters": {"type": "object", "properties": {"path": {"type": "string"}}, "required": []}}},
        {"type": "function", "function": {"name": "bash", "description": "Ejecuta comando permitido (ls, cat, echo, pwd, cargo --version, rustc --version, cargo check, cargo build, cargo test, cargo run, cargo clippy, cargo fmt, git con política por subcomando) en el workspace.", "parameters": {"type": "object", "properties": {"cmd": {"type": "string"}}, "required": ["cmd"]}}},
        {"type": "function", "function": {"name": "search_files", "description": "Busca texto en el workspace. Devuelve máx 20 bloques path:línea con 3 líneas de contexto, rankeados. Nunca archivos completos. Ignora target/, .git/, node_modules/, *.lock.", "parameters": {"type": "object", "properties": {"query": {"type": "string"}, "dir": {"type": "string"}}, "required": ["query"]}}},
        {"type": "function", "function": {"name": "get_file_outline", "description": "Solo firmas de un archivo (funciones, structs, imports) para decidir QUÉ leer. Úsalo antes de read_file en archivos grandes.", "parameters": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}}},
        {"type": "function", "function": {"name": "fetch_url", "description": "Descarga una URL http(s) como texto (tope 64k). Dominios no autorizados piden permiso.", "parameters": {"type": "object", "properties": {"url": {"type": "string"}}, "required": ["url"]}}}
    ])
}

/// Schemas Anthropic (input_schema en vez de parameters).
pub fn anthropic_schemas() -> serde_json::Value {
    serde_json::json!([
        {"name": "read_file", "description": "Lee un archivo del workspace POR PÁGINAS (200 líneas por defecto, offset 1-based).", "input_schema": {"type": "object", "properties": {"path": {"type": "string"}, "offset": {"type": "integer"}, "limit": {"type": "integer"}}, "required": ["path"]}},
        {"name": "write_file", "description": "Crea o sobrescribe un archivo del workspace.", "input_schema": {"type": "object", "properties": {"path": {"type": "string"}, "content": {"type": "string"}}, "required": ["path", "content"]}},
        {"name": "edit_file", "description": "Reemplaza EXACTAMENTE 1 ocurrencia de `old` por `new`.", "input_schema": {"type": "object", "properties": {"path": {"type": "string"}, "old": {"type": "string"}, "new": {"type": "string"}}, "required": ["path", "old", "new"]}},
        {"name": "delete_file", "description": "Borra un archivo del workspace.", "input_schema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}},
        {"name": "list_dir", "description": "Lista una carpeta del workspace.", "input_schema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": []}},
        {"name": "bash", "description": "Ejecuta comando permitido en el workspace.", "input_schema": {"type": "object", "properties": {"cmd": {"type": "string"}}, "required": ["cmd"]}},
        {"name": "search_files", "description": "Busca texto en el workspace: máx 20 bloques con contexto, rankeados. Ignora target/, .git/, node_modules/, *.lock.", "input_schema": {"type": "object", "properties": {"query": {"type": "string"}, "dir": {"type": "string"}}, "required": ["query"]}},
        {"name": "get_file_outline", "description": "Solo firmas de un archivo (funciones, structs, imports) para decidir QUÉ leer.", "input_schema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}},
        {"name": "fetch_url", "description": "Descarga una URL http(s) como texto (tope 64k).", "input_schema": {"type": "object", "properties": {"url": {"type": "string"}}, "required": ["url"]}}
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_ws(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("arqhia-tools-test-{name}"));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p.canonicalize().unwrap()
    }

    #[tokio::test]
    async fn guard_rejects_escape() {
        let ws = tmp_ws("escape");
        assert!(resolve(&ws, &[], "../fuera.txt").is_err());
        assert!(resolve(&ws, &[], "/etc/passwd").is_err());
        assert!(resolve(&ws, &[], "ok/sub.txt").is_ok());
    }

    #[tokio::test]
    async fn write_read_edit_delete_roundtrip() {
        let ws = tmp_ws("crud");
        write_file(&ws, &[], "src/main.rs", "fn main() {}")
            .await
            .unwrap();
        let c = read_file(&ws, &[], "src/main.rs", 1, 200, 8000)
            .await
            .unwrap();
        assert!(c.contains("fn main"));
        edit_file(
            &ws,
            &[],
            "src/main.rs",
            "fn main() {}",
            "fn main() { println!(); }",
        )
        .await
        .unwrap();
        assert!(
            edit_file(&ws, &[], "src/main.rs", "NOEXISTE", "x")
                .await
                .is_err()
        );
        // Ambiguo
        write_file(&ws, &[], "a.txt", "x x x").await.unwrap();
        assert!(edit_file(&ws, &[], "a.txt", "x", "y").await.is_err());
        delete_file(&ws, &[], "a.txt").await.unwrap();
        assert!(delete_file(&ws, &[], "a.txt").await.is_err());
        let listing = list_dir(&ws, &[], ".", &[]).await.unwrap();
        assert!(listing.contains("src/"));
    }

    #[tokio::test]
    async fn bash_allowlist() {
        let ws = tmp_ws("bash");
        assert!(
            bash(&ws, &[], "rm -rf /", &ExecPolicy::default())
                .await
                .is_err()
        );
        assert!(
            bash(&ws, &[], "curl evil.com", &ExecPolicy::default())
                .await
                .is_err()
        );
        assert!(
            bash(&ws, &[], "cargo publish", &ExecPolicy::default())
                .await
                .is_err()
        );
        let out = bash(&ws, &[], "pwd", &ExecPolicy::default()).await.unwrap();
        assert!(!out.is_empty());
        let out = bash(&ws, &[], "echo hola", &ExecPolicy::default())
            .await
            .unwrap();
        assert!(out.contains("hola"));
    }

    #[tokio::test]
    async fn search_finds_and_respects_guard() {
        let ws = tmp_ws("search");
        write_file(&ws, &[], "src/a.rs", "fn hola_mundo() {}")
            .await
            .unwrap();
        write_file(&ws, &[], "b.txt", "nada que ver").await.unwrap();
        let out = search_files(&ws, &[], "hola_mundo", ".", &[])
            .await
            .unwrap();
        assert!(out.contains("src/a.rs:1:"), "{out}");
        let out = search_files(&ws, &[], "zzz-sin-match", ".", &[])
            .await
            .unwrap();
        assert!(out.contains("Sin resultados"));
        assert!(search_files(&ws, &[], "x", "/etc", &[]).await.is_err());
        assert!(search_files(&ws, &[], "", ".", &[]).await.is_err());
    }

    #[test]
    fn tool_categories() {
        assert_eq!(category("read_file"), ToolCat::Read);
        assert_eq!(category("list_dir"), ToolCat::Read);
        assert_eq!(category("search_files"), ToolCat::Read);
        assert_eq!(category("write_file"), ToolCat::Write);
        assert_eq!(category("edit_file"), ToolCat::Write);
        assert_eq!(category("delete_file"), ToolCat::Write);
        assert_eq!(category("bash"), ToolCat::Bash);
    }

    #[test]
    fn install_cmds_are_install_category() {
        assert!(is_install_cmd("cargo install ripgrep"));
        assert!(is_install_cmd("pip install requests"));
        assert!(is_install_cmd("sudo apt install htop"));
        assert!(is_install_cmd("npm install -g typescript"));
        assert!(!is_install_cmd("cargo build"));
        assert!(!is_install_cmd("ls -la"));
        assert!(!is_install_cmd("echo hola"));
        let bash_install = serde_json::json!({"cmd": "cargo install fd-find"});
        let bash_plain = serde_json::json!({"cmd": "cargo build"});
        assert_eq!(category_of_call("bash", &bash_install), ToolCat::Install);
        assert_eq!(category_of_call("bash", &bash_plain), ToolCat::Bash);
        assert_eq!(
            category_of_call("fetch_url", &serde_json::json!({})),
            ToolCat::Net
        );
    }

    #[test]
    fn git_policy_classifies_and_blocks() {
        assert_eq!(classify_git("git status"), Some(GitKind::Read));
        assert_eq!(classify_git("git diff --stat"), Some(GitKind::Read));
        assert_eq!(classify_git("git remote -v"), Some(GitKind::Read));
        assert_eq!(classify_git("git add -A"), Some(GitKind::Write));
        assert_eq!(classify_git("git commit -m x"), Some(GitKind::Write));
        assert_eq!(classify_git("git push origin ARQHIA"), Some(GitKind::Net));
        assert_eq!(
            classify_git("git push --force origin x"),
            Some(GitKind::Blocked)
        );
        assert_eq!(
            classify_git("git reset --hard HEAD"),
            Some(GitKind::Blocked)
        );
        assert_eq!(classify_git("git clean -fd"), Some(GitKind::Blocked));
        assert_eq!(classify_git("git rebase main"), Some(GitKind::Blocked));
        assert_eq!(
            classify_git("git config user.name x"),
            Some(GitKind::Blocked)
        );
        assert_eq!(
            classify_git("git remote add origin url"),
            Some(GitKind::Blocked)
        );
        assert_eq!(classify_git("git -C /etc status"), Some(GitKind::Blocked));
        assert_eq!(classify_git("ls -la"), None);
        // Categorías de permiso para el panel.
        let add = serde_json::json!({"cmd": "git add -A"});
        let push = serde_json::json!({"cmd": "git push origin ARQHIA"});
        assert_eq!(category_of_call("bash", &add), ToolCat::Git);
        assert_eq!(category_of_call("bash", &push), ToolCat::GitPush);
        // Allowlist deja pasar git + clippy/fmt; el executor decide política.
        assert!(is_allowed("git status"));
        assert!(is_allowed("cargo clippy --all-targets"));
        assert!(is_allowed("cargo fmt"));
    }

    #[tokio::test]
    async fn bash_git_blocks_destructive_and_gates_policy() {
        let ws = tmp_ws("git-policy");
        let policy = ExecPolicy::default();
        for cmd in [
            "git push --force",
            "git reset --hard HEAD",
            "git clean -fd",
            "git rebase main",
            "git config user.name x",
        ] {
            let err = bash(&ws, &[], cmd, &policy).await.unwrap_err();
            assert!(err.contains("bloqueado"), "{cmd}: {err}");
        }
        // Write con autonomía ReadOnly: rechazado por política.
        let ro = ExecPolicy {
            git_autonomy: GitAutonomy::ReadOnly,
            ..ExecPolicy::default()
        };
        let err = bash(&ws, &[], "git add -A", &ro).await.unwrap_err();
        assert!(err.contains("no permitida"), "{err}");
        // Push sin push_enabled: rechazado por política (aunque CommitLocal).
        let err = bash(&ws, &[], "git push origin ARQHIA", &policy)
            .await
            .unwrap_err();
        assert!(err.contains("no permitida"), "{err}");
        // CommitAndPush + push_enabled: el gate pasa (falla luego git, no política).
        let ap = ExecPolicy {
            git_autonomy: GitAutonomy::CommitAndPush,
            git_push_enabled: true,
            ..ExecPolicy::default()
        };
        assert!(bash(&ws, &[], "git pull", &ap).await.is_ok());
    }

    #[test]
    fn url_domain_allowlist() {
        let doms = vec!["example.com".to_string()];
        assert!(url_domain_listed("https://example.com/x", &doms));
        assert!(url_domain_listed("https://sub.example.com/x", &doms));
        assert!(!url_domain_listed("https://evil-example.com/x", &doms));
        assert!(!url_domain_listed("https://evil.com/x", &doms));
        assert!(!url_domain_listed("ftp://example.com/x", &doms));
        assert!(!url_domain_listed("no-es-url", &doms));
        assert!(!url_domain_listed("https://example.com/x", &[]));
    }

    #[test]
    fn truncate_never_splits_utf8() {
        let s = "diseño ñandú 🚀".repeat(10);
        let cut = truncate(&s, 20);
        assert_eq!(
            cut.chars().count(),
            20 + "…[truncado 110 chars]".chars().count()
        );
        assert!(cut.starts_with("diseño ñandú 🚀"));
    }

    #[tokio::test]
    async fn extra_paths_guard() {
        let ws = tmp_ws("extra-ws");
        let outside = tmp_ws("extra-out");
        std::fs::write(outside.join("dato.txt"), "secreto").unwrap();
        // Sin extra: fuera rechazado.
        assert!(
            read_file(
                &ws,
                &[],
                "../arqhia-tools-test-extra-out/dato.txt",
                1,
                200,
                8000
            )
            .await
            .is_err()
        );
        // Con extra explícito: permitido.
        let c = read_file(
            &ws,
            std::slice::from_ref(&outside),
            "../arqhia-tools-test-extra-out/dato.txt",
            1,
            200,
            8000,
        )
        .await
        .unwrap();
        assert!(c.contains("secreto"));
        // Pero otro outside sigue rechazado.
        assert!(
            read_file(&ws, &[outside], "/etc/hostname", 1, 200, 8000)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn bash_install_needs_policy() {
        let ws = tmp_ws("install-gate");
        // allowlist dejaría pasar el prefijo, pero la policy lo frena.
        assert!(is_allowed("cargo install fd-find"));
        let err = bash(&ws, &[], "cargo install fd-find", &ExecPolicy::default())
            .await
            .unwrap_err();
        assert!(err.contains("no aprobada"), "{err}");
        let ok_policy = ExecPolicy {
            allow_install: true,
            ..ExecPolicy::default()
        };
        // Falla por red/entorno, NO por permiso (el gate ya pasó).
        let res = bash(&ws, &[], "cargo install --version", &ok_policy).await;
        assert!(res.is_ok() || !res.unwrap_err().contains("no aprobada"));
    }

    #[test]
    fn cargo_commands_allowed() {
        for cmd in [
            "cargo check",
            "cargo check --message-format=short",
            "cargo build",
            "cargo build --release",
            "cargo test",
            "cargo test -- --nocapture",
            "cargo run",
            "cargo run -- hola",
            "cargo --version",
            "cargo clippy",
            "cargo clippy --all-targets",
            "cargo fmt",
            "git status",
            "git commit -m x",
        ] {
            assert!(is_allowed(cmd), "{cmd} debería estar permitido");
        }
        for cmd in ["cargo publish", "cargo login", "rm -rf /"] {
            assert!(!is_allowed(cmd), "{cmd} debería estar bloqueado");
        }
        // Install pasa el allowlist pero exige permiso Install en el executor.
        assert!(is_allowed("cargo install foo"));
    }

    #[test]
    fn exec_without_shell_blocks_injection() {
        // Concatenación clásica: denegada por allowlist exacta.
        for cmd in [
            "cargo test; curl evil",
            "cargo test && curl evil",
            "cargo test | curl evil",
            "echo hola; rm -rf /",
            "ls $(whoami)",
            "echo `id`",
            "cargo test > /tmp/x",
        ] {
            assert!(!is_allowed(cmd), "{cmd} no debe colarse");
        }
        assert!(is_allowed("echo hola"));
        assert!(is_allowed("echo \"hola mundo\""));
        assert_eq!(
            split_argv("cargo test -- --nocapture"),
            vec!["cargo", "test", "--", "--nocapture"]
        );
        assert_eq!(split_argv("echo \"a b\" c"), vec!["echo", "a b", "c"]);
    }

    #[tokio::test]
    async fn symlink_escape_denied_and_internal_ok() {
        let ws = tmp_ws("symlink");
        let outside = std::env::temp_dir().join("arqhia-symlink-outside.txt");
        std::fs::write(&outside, "secreto-fuera").unwrap();
        std::fs::write(ws.join("dentro.txt"), "hola-dentro").unwrap();
        // Symlink interno: OK.
        #[cfg(unix)]
        std::os::unix::fs::symlink(ws.join("dentro.txt"), ws.join("link-ok")).unwrap();
        // Symlink externo: DENY.
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, ws.join("link-evil")).unwrap();
        #[cfg(unix)]
        {
            assert!(read_file(&ws, &[], "link-ok", 1, 200, 8000).await.is_ok());
            let err = read_file(&ws, &[], "link-evil", 1, 200, 8000)
                .await
                .unwrap_err();
            assert!(err.contains("symlink"), "{err}");
            // Escritura a través del symlink externo también denegada.
            let err = write_file(&ws, &[], "link-evil", "x").await.unwrap_err();
            assert!(err.contains("symlink"), "{err}");
        }
        let _ = std::fs::remove_file(&outside);
    }

    #[test]
    fn ssrf_hosts_blocked() {
        for h in [
            "localhost",
            "127.0.0.1",
            "127.0.0.2",
            "0.0.0.0",
            "::1",
            "169.254.169.254",
            "169.254.10.20",
            // v0.9.5: privadas, CGNAT y equivalentes IPv6 también se bloquean.
            "10.0.0.1",
            "192.168.1.10",
            "172.16.5.4",
            "100.64.0.1",
            "::ffff:127.0.0.1",
            "fc00::1",
            "fe80::1",
        ] {
            assert!(is_ssrf_host(h), "{h} debe bloquearse");
        }
        // Con puerto (IPv4) también.
        for h in ["127.0.0.1", "169.254.169.254", "192.168.0.1"] {
            assert!(
                is_ssrf_host(&format!("{h}:8000")),
                "{h} con puerto debe bloquearse"
            );
        }
        // Userinfo no debe permitir el bypass `http://evil.com@127.0.0.1/`.
        assert!(is_ssrf_host("evil.com@127.0.0.1"));
        assert!(is_ssrf_host(&url_host("http://evil.com@127.0.0.1/")));
        assert!(!is_ssrf_host("example.com"));
        assert!(!is_ssrf_host("8.8.8.8"));
    }

    #[test]
    fn url_host_strips_userinfo_and_port() {
        assert_eq!(url_host("https://example.com/x"), "example.com");
        assert_eq!(url_host("http://example.com:8080/x"), "example.com");
        assert_eq!(url_host("http://user:pass@example.com/x"), "example.com");
        assert_eq!(url_host("http://evil.com@127.0.0.1/"), "127.0.0.1");
        assert_eq!(url_host("https://[::1]:8000/x"), "::1");
        assert_eq!(url_host("http://127.0.0.1./x"), "127.0.0.1");
        assert_eq!(url_host("ftp://example.com/x"), "");
        assert_eq!(url_host("no-es-url"), "");
    }

    #[tokio::test]
    async fn fetch_url_blocks_ssrf_without_request() {
        let policy = ExecPolicy {
            net_approved: true,
            ..ExecPolicy::default()
        };
        let err = fetch_url("http://127.0.0.1/", &policy).await.unwrap_err();
        assert!(err.contains("SSRF") || err.contains("bloqueado"), "{err}");
        let err = fetch_url("http://localhost:8000/x", &policy)
            .await
            .unwrap_err();
        assert!(err.contains("SSRF") || err.contains("bloqueado"), "{err}");
        let err = fetch_url("http://169.254.169.254/latest/meta-data/", &policy)
            .await
            .unwrap_err();
        assert!(err.contains("SSRF") || err.contains("bloqueado"), "{err}");
    }

    #[test]
    fn ignores_cover_build_vcs_deps_and_locks() {
        let def = default_ignores();
        assert!(is_ignored("target/debug/app", &def));
        assert!(is_ignored("src/target/x.rs", &def));
        assert!(is_ignored(".git/objects/xx", &def));
        assert!(is_ignored("node_modules/react/index.js", &def));
        assert!(is_ignored("Cargo.lock", &def));
        assert!(is_ignored("sub/Cargo.lock", &def));
        assert!(!is_ignored("src/main.rs", &def));
        assert!(!is_ignored("Cargo.toml", &def));
        // Lista vacía = defaults (nunca peta por config sin migrar).
        assert!(is_ignored("target/x", &[]));
        // Personalizados sustituyen.
        assert!(is_ignored("dist/bundle.js", &["dist/".to_string()]));
        assert!(!is_ignored("target/x", &["dist/".to_string()]));
    }

    #[tokio::test]
    async fn read_pagination_reports_ranges() {
        let ws = tmp_ws("paged");
        let body = (1..=10)
            .map(|i| format!("línea {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        write_file(&ws, &[], "doc.txt", &body).await.unwrap();
        let p1 = read_file(&ws, &[], "doc.txt", 1, 4, 8000).await.unwrap();
        assert!(p1.contains("líneas 1–4 de 10 total"), "{p1}");
        assert!(p1.contains("1| línea 1"), "{p1}");
        assert!(p1.contains("siguen 6 líneas (pide offset 5)"), "{p1}");
        let p2 = read_file(&ws, &[], "doc.txt", 9, 200, 8000).await.unwrap();
        assert!(p2.contains("líneas 9–10 de 10 total"), "{p2}");
        assert!(!p2.contains("siguen"), "{p2}");
        assert!(read_file(&ws, &[], "doc.txt", 99, 10, 8000).await.is_err());
        let empty = tmp_ws("paged-empty");
        write_file(&empty, &[], "v.txt", "").await.unwrap();
        let e = read_file(&empty, &[], "v.txt", 1, 200, 8000).await.unwrap();
        assert!(e.contains("vacío"), "{e}");
    }

    #[tokio::test]
    async fn search_v2_ranks_and_ignores() {
        let ws = tmp_ws("search-v2");
        // Nombre de archivo con la query rankea primero.
        write_file(
            &ws,
            &[],
            "auth_handler.rs",
            "fn login() {}\n// login aquí\nmás login",
        )
        .await
        .unwrap();
        write_file(&ws, &[], "notas.txt", "hablar del login mañana")
            .await
            .unwrap();
        // Ignorados: target/ y *.lock no aparecen aunque matcheen.
        write_file(&ws, &[], "target/login_fake.rs", "login login login")
            .await
            .unwrap();
        write_file(&ws, &[], "Cargo.lock", "login-fake-package")
            .await
            .unwrap();
        let out = search_files(&ws, &[], "login", ".", &[]).await.unwrap();
        let first = out.lines().next().unwrap_or("");
        assert!(
            first.starts_with("auth_handler.rs:"),
            "ranking por nombre: {out}"
        );
        assert!(!out.contains("target/"), "target/ ignorado: {out}");
        assert!(!out.contains("Cargo.lock"), "*.lock ignorado: {out}");
        // Contexto con marcadores » «, nunca el archivo completo.
        assert!(
            out.contains('»') || out.contains('«'),
            "bloques con contexto: {out}"
        );
        assert!(!out.contains("más login\nmás"), "{out}");
    }

    #[tokio::test]
    async fn outline_lists_signatures_not_bodies() {
        let ws = tmp_ws("outline");
        let code = "use std::fs;\n\n/// Suma dos cosas\npub fn sumar(a: u32, b: u32) -> u32 {\n    let tmp = a + b;\n    tmp\n}\n\npub struct Punto { x: f64 }\n\nfn main() { println!(\"hola\"); }\n";
        write_file(&ws, &[], "lib.rs", code).await.unwrap();
        let out = get_file_outline(&ws, &[], "lib.rs").await.unwrap();
        assert!(out.contains("outline de lib.rs"), "{out}");
        assert!(out.contains("fn sumar"), "{out}");
        assert!(out.contains("struct Punto"), "{out}");
        assert!(out.contains("fn main"), "{out}");
        assert!(!out.contains("let tmp"), "cuerpos fuera: {out}");
        assert!(get_file_outline(&ws, &[], "no-existe.rs").await.is_err());
    }

    #[test]
    fn outline_sig_heuristics() {
        assert!(outline_sig("pub fn hola() {").is_some());
        assert!(outline_sig("  struct Punto {").is_some());
        assert!(outline_sig("use std::fs;").is_some());
        assert!(outline_sig("import os").is_some());
        assert!(outline_sig("def entrenar():").is_some());
        assert!(outline_sig("## Diseño").is_some());
        assert!(outline_sig("    let x = 1;").is_none());
        assert!(outline_sig("").is_none());
        assert!(outline_sig("// comentario").is_none());
    }

    #[test]
    fn schemas_declare_pagination_and_outline() {
        let openai = openai_schemas();
        let tools = openai.as_array().unwrap();
        assert!(
            tools
                .iter()
                .any(|t| t["function"]["name"] == "get_file_outline")
        );
        let read = tools
            .iter()
            .find(|t| t["function"]["name"] == "read_file")
            .unwrap();
        let props = &read["function"]["parameters"]["properties"];
        assert!(props.get("offset").is_some() && props.get("limit").is_some());
        let anth = anthropic_schemas();
        assert!(
            anth.as_array()
                .unwrap()
                .iter()
                .any(|t| t["name"] == "get_file_outline")
        );
        assert_eq!(category("get_file_outline"), ToolCat::Read);
    }

    #[test]
    fn mcp_category_and_schemas() {
        use crate::config::{McpServerConfig, McpTransport};
        use std::collections::HashMap;
        // auto=false -> Net (pide permiso), auto=true -> Read (auto), npx -> Install
        let mut servers = HashMap::new();
        servers.insert(
            "docs".to_string(),
            McpServerConfig {
                transport: McpTransport::Stdio,
                command: "my-server".to_string(),
                args: vec![],
                url: "".to_string(),
                auto: false,
                timeout_s: 10,
            },
        );
        assert_eq!(
            category_of_mcp_call("mcp__docs__echo", &servers),
            ToolCat::Net
        );
        servers.get_mut("docs").unwrap().auto = true;
        assert_eq!(
            category_of_mcp_call("mcp__docs__echo", &servers),
            ToolCat::Read
        );
        servers.get_mut("docs").unwrap().command = "npx".to_string();
        assert_eq!(
            category_of_mcp_call("mcp__docs__echo", &servers),
            ToolCat::Install
        );
        // Schemas anexados
        let tool = crate::mcp::McpTool {
            name: "echo".to_string(),
            description: "hi".to_string(),
            input_schema: serde_json::json!({"type":"object","properties":{"text":{"type":"string"}}}),
        };
        let mut coll = HashMap::new();
        coll.insert("docs".to_string(), vec![tool]);
        let open = mcp_openai_schemas(&coll);
        assert_eq!(open.as_array().unwrap().len(), 1);
        assert_eq!(open[0]["function"]["name"], "mcp__docs__echo");
        assert!(
            open[0]["function"]["description"]
                .as_str()
                .unwrap()
                .contains("[MCP docs]")
        );
        let with = openai_schemas_with_mcp(&coll);
        assert!(with.as_array().unwrap().len() > 9);
    }

    #[tokio::test]
    async fn mcp_e2e_via_execute_with_stub() {
        crate::mcp::clear_cache();
        let dir = std::env::temp_dir().join("arqhia-mcp-tools-e2e");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("stub.py");
        std::fs::write(&script, r#"import sys, json
for line in sys.stdin:
    line=line.strip()
    if not line: continue
    try: req=json.loads(line)
    except: continue
    mid=req.get("id")
    method=req.get("method")
    if method=="initialize":
        sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"protocolVersion":"2024-11-05","capabilities":{}}})+"\n"); sys.stdout.flush()
    elif method=="notifications/initialized": continue
    elif method=="tools/list":
        sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"tools":[{"name":"echo","description":"echo","inputSchema":{"type":"object","properties":{"text":{"type":"string"}}}}]}})+"\n"); sys.stdout.flush()
    elif method=="tools/call":
        args=req.get("params",{}).get("arguments",{}); txt=args.get("text","")
        sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"content":[{"type":"text","text":f"echo:{txt}"}]}})+"\n"); sys.stdout.flush()
"#).unwrap();
        let ws = tmp_ws("mcp-e2e");
        let mut servers = std::collections::HashMap::new();
        servers.insert(
            "stub".to_string(),
            crate::config::McpServerConfig {
                transport: crate::config::McpTransport::Stdio,
                command: "python3".to_string(),
                args: vec![script.to_string_lossy().to_string()],
                url: "".to_string(),
                auto: false,
                timeout_s: 10,
            },
        );
        // list via mcp direct
        let tools = crate::mcp::list_tools_stdio("stub", servers.get("stub").unwrap())
            .await
            .unwrap();
        assert_eq!(tools.len(), 1);
        // execute via tools::execute with Install gating OFF (should pass because command is python3, not npx)
        let mut policy = ExecPolicy {
            mcp_servers: servers.clone(),
            ..ExecPolicy::default()
        };
        policy.allow_install = true; // for stdio we don't need Install, but set true
        let out = execute(
            &ws,
            &[],
            "mcp__stub__echo",
            &serde_json::json!({"text":"hola"}),
            &policy,
        )
        .await;
        assert!(out.contains("echo:hola"), "{out}");
        // npx server requires Install -> without allow_install should fail
        let mut npx_servers = std::collections::HashMap::new();
        npx_servers.insert(
            "npx-srv".to_string(),
            crate::config::McpServerConfig {
                transport: crate::config::McpTransport::Stdio,
                command: "npx".to_string(),
                args: vec![],
                url: "".to_string(),
                auto: false,
                timeout_s: 10,
            },
        );
        let pol2 = ExecPolicy {
            mcp_servers: npx_servers,
            allow_install: false,
            ..ExecPolicy::default()
        };
        let err = execute(
            &ws,
            &[],
            "mcp__npx-srv__echo",
            &serde_json::json!({}),
            &pol2,
        )
        .await;
        assert!(err.contains("Install"), "{err}");
        crate::mcp::clear_cache();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
