//! Import auto (v0.8.1): detectar código existente y pre-rellenar.
//!
//! Puro salvo `scan` (lee el filesystem con topes, ignora
//! `target/.git/node_modules/*.lock`).

use super::levels::{Categoria, Plataforma, StackOpt, SysType};
use super::Answers;

/// Resultado del escaneo de una carpeta con posible código.
#[derive(Debug, Clone, Default)]
pub struct ImportScan {
    pub files: usize,
    pub languages: Vec<String>,
    pub stacks: Vec<StackOpt>,
    pub entrypoints: Vec<String>,
    pub has_readme: bool,
    pub has_specs: bool,
    pub has_tests: bool,
}

const MANIFESTS: [(&str, StackOpt, &str); 6] = [
    ("Cargo.toml", StackOpt::Rust, "Rust"),
    ("package.json", StackOpt::JsTs, "JS/TS"),
    ("go.mod", StackOpt::Go, "Go"),
    ("pyproject.toml", StackOpt::Python, "Python"),
    ("requirements.txt", StackOpt::Python, "Python"),
    ("Dockerfile", StackOpt::Otro, "Docker"),
];

/// Escanea 1 nivel + manifiestos conocidos. Nunca falla.
pub fn scan(dir: &std::path::Path) -> ImportScan {
    let mut out = ImportScan::default();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    let mut names: Vec<String> = Vec::new();
    for e in entries.flatten().take(60) {
        let name = e.file_name().to_string_lossy().to_string();
        if matches!(name.as_str(), "target" | ".git" | "node_modules") {
            continue;
        }
        if name.ends_with(".lock") {
            continue;
        }
        names.push(name);
    }
    out.files = names.len();
    for (manifest, stack, lang) in MANIFESTS {
        if names.iter().any(|n| n == manifest) {
            if !out.stacks.contains(&stack) {
                out.stacks.push(stack);
            }
            if !out.languages.contains(&lang.to_string()) {
                out.languages.push(lang.to_string());
            }
        }
    }
    // Heurística por extensiones si no hubo manifiesto.
    if out.stacks.is_empty() {
        let joined = names.join(" ");
        let hints = [
            (".rs", StackOpt::Rust, "Rust"),
            (".py", StackOpt::Python, "Python"),
            (".go", StackOpt::Go, "Go"),
            (".ts", StackOpt::JsTs, "JS/TS"),
        ];
        for (ext, stack, lang) in hints {
            if joined.contains(ext) {
                out.stacks.push(stack);
                out.languages.push(lang.to_string());
                break;
            }
        }
    }
    for n in &names {
        let low = n.to_lowercase();
        if low.starts_with("readme") {
            out.has_readme = true;
        }
        if low == "specs.md" || low == "espec.md" {
            out.has_specs = true;
        }
        if n == "Cargo.toml" || n == "package.json" || n == "go.mod" || n == "main.py" {
            out.entrypoints.push(n.clone());
        }
    }
    out.has_tests = names.iter().any(|n| n == "tests" || n.contains("test"));
    out
}

/// Campos CONTEXT que faltan según la plantilla (gap vs PROJECT/SPECS).
pub fn gap_fields(scan: &ImportScan) -> Vec<&'static str> {
    let mut gaps = Vec::new();
    if !scan.has_readme {
        gaps.push("descripcion");
    }
    if !scan.has_specs {
        gaps.push("objetivo/funcionalidades");
    }
    if scan.stacks.is_empty() {
        gaps.push("stack");
    }
    gaps
}

/// Preguntas estáticas de huecos (fallback sin provider).
pub fn gap_fallback(scan: &ImportScan) -> Vec<String> {
    let mut out = Vec::new();
    if !scan.has_readme {
        out.push("¿Qué hace este código en una o dos frases?".to_string());
    }
    if !scan.has_specs {
        out.push("¿Qué problema resuelve y qué funcionalidades ya existen?".to_string());
    }
    if scan.stacks.is_empty() {
        out.push("¿Con qué lenguaje/stack está construido?".to_string());
    }
    if out.is_empty() {
        out.push("¿Qué falta para considerar este proyecto completo?".to_string());
    }
    out.truncate(5);
    out
}

/// Pre-rellena respuestas desde el escaneo (merge sin borrar: solo
/// completa vacíos, nunca pisa lo que el usuario ya escribió).
pub fn prefill_answers(base: &mut Answers, folder_name: &str, scan: &ImportScan) {
    if base.nombre.trim().is_empty() {
        base.nombre = folder_name.to_string();
    }
    for s in &scan.stacks {
        if !base.stacks.contains(s) {
            base.stacks.push(*s);
        }
    }
    if !scan.languages.is_empty() && base.descripcion.trim().is_empty() {
        base.descripcion = format!("Proyecto importado (detectado: {}).", scan.languages.join(", "));
    }
    // Plataforma por defecto según lo detectado (editable).
    if base.plataformas.is_empty() {
        base.plataformas = vec![Plataforma::ServidorNube];
    }
    let _ = Categoria::Aplicacion;
    let _ = SysType::AppWeb;
}

/// Línea de banner para el wizard en modo import.
pub fn import_banner(folder: &str, scan: &ImportScan) -> String {
    let lang = if scan.languages.is_empty() {
        "lenguaje sin detectar".to_string()
    } else {
        scan.languages.join(" + ")
    };
    format!(
        "Detectamos {lang} + {} archivos en {folder}; README {} / SPECS {}. Solo preguntamos lo que falta.",
        scan.files,
        if scan.has_readme { "sí" } else { "no" },
        if scan.has_specs { "sí" } else { "no" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_detects_rust_manifest_and_prefills() {
        let base = std::env::temp_dir().join("arqhia-import-test");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("Cargo.toml"), "[package]\nname = \"x\"").unwrap();
        std::fs::write(base.join("README.md"), "# X").unwrap();
        let scan = scan(&base);
        assert!(scan.stacks.contains(&StackOpt::Rust));
        assert!(scan.has_readme && !scan.has_specs);
        let mut a = Answers::default();
        prefill_answers(&mut a, "mi-proyecto", &scan);
        assert_eq!(a.nombre, "mi-proyecto");
        assert!(a.stacks.contains(&StackOpt::Rust));
        assert!(!gap_fields(&scan).contains(&"stack"));
        assert!(!gap_fallback(&scan).is_empty());
        assert!(import_banner("mi-proyecto", &scan).contains("Rust"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn scan_empty_dir_has_gaps() {
        let base = std::env::temp_dir().join("arqhia-import-empty");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let scan = scan(&base);
        assert!(scan.stacks.is_empty());
        assert!(!gap_fields(&scan).is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }
}
