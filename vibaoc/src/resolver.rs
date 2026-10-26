use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::diagnostic::Locale;
use vibao_ast::{ComponentDef, ImportDecl, Page, Program};

#[derive(Debug)]
pub struct ResolverError {
    pub message: String,
    pub path: String,
    pub source: String,
    pub line: usize,
    pub column: usize,
    pub locale: Locale,
}

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[resolver] {}", self.message)
    }
}
impl std::error::Error for ResolverError {}

fn err_at(
    message: impl Into<String>,
    path: &Path,
    source: &str,
    line: usize,
    column: usize,
    locale: Locale,
) -> ResolverError {
    ResolverError {
        message: message.into(),
        path: path.display().to_string(),
        source: source.to_string(),
        line: line.max(1),
        column: column.max(1),
        locale,
    }
}

#[allow(dead_code)]
pub fn resolve(program: Program, root_path: &Path) -> Result<Program, ResolverError> {
    resolve_with_locale(program, root_path, Locale::English)
}

pub fn resolve_with_locale(
    mut program: Program,
    root_path: &Path,
    root_locale: Locale,
) -> Result<Program, ResolverError> {
    let mut visiting: HashSet<PathBuf> = HashSet::new();
    let mut merged: HashSet<PathBuf> = HashSet::new();
    let root_canonical = canonicalize_best_effort(root_path);
    visiting.insert(root_canonical);

    let mut all_components: Vec<ComponentDef> = std::mem::take(&mut program.app.components);
    let mut all_pages: Vec<Page> = std::mem::take(&mut program.app.pages);

    for import in &program.app.imports {
        resolve_one_import(
            import,
            root_path,
            root_locale,
            &mut visiting,
            &mut merged,
            &mut all_components,
            &mut all_pages,
        )?;
    }

    program.app.components = all_components;
    program.app.pages = all_pages;
    program.app.imports = Vec::new();

    Ok(program)
}

fn resolve_one_import(
    import: &ImportDecl,
    importing_file: &Path,
    importing_locale: Locale,
    visiting: &mut HashSet<PathBuf>,
    merged: &mut HashSet<PathBuf>,
    all_components: &mut Vec<ComponentDef>,
    all_pages: &mut Vec<Page>,
) -> Result<(), ResolverError> {
    let resolved_path = resolve_import_path(importing_file, &import.path, importing_locale)?;
    let canonical = canonicalize_best_effort(&resolved_path);

    if merged.contains(&canonical) {
        return Ok(());
    }

    if visiting.contains(&canonical) {
        return Err(err_at(
            format!(
                "Circular import detected for \"{}\". The import chain eventually imports this file again.",
                import.path
            ),
            importing_file,
            "",
            import.pos.line,
            import.pos.column,
            importing_locale,
        ));
    }

    let source = std::fs::read_to_string(&resolved_path).map_err(|e| {
        err_at(
            format!("Could not read imported file \"{}\": {}", import.path, e),
            &resolved_path,
            "",
            import.pos.line,
            import.pos.column,
            importing_locale,
        )
    })?;

    let imported_locale = Locale::detect_from_source(&source);
    let tokens = crate::lexer::tokenize(&source).map_err(|e| {
        err_at(
            format!("Lexer error in imported file: {}", e.message),
            &resolved_path,
            &source,
            e.line,
            e.column,
            imported_locale,
        )
    })?;

    let mut parser = crate::parser::Parser::new(tokens);
    let module_body = match parser.parse_module_file() {
        Ok(body) => body,
        Err(e) => {
            let locale = parser.locale();
            return Err(err_at(
                format!("Parser error in imported file: {}", e.message),
                &resolved_path,
                &source,
                e.line,
                e.column,
                locale,
            ));
        }
    };

    let defined_names: HashSet<&str> = module_body
        .components
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    for wanted in &import.names {
        if !defined_names.contains(wanted.as_str()) {
            return Err(err_at(
                format!(
                    "Imported component \"{}\" was not found in \"{}\". Available components: {}",
                    wanted,
                    resolved_path.display(),
                    if defined_names.is_empty() {
                        "(none)".to_string()
                    } else {
                        defined_names.iter().copied().collect::<Vec<_>>().join(", ")
                    }
                ),
                importing_file,
                &source,
                import.pos.line,
                import.pos.column,
                parser.locale(),
            ));
        }
    }

    visiting.insert(canonical.clone());
    for nested_import in &module_body.imports {
        resolve_one_import(
            nested_import,
            &resolved_path,
            parser.locale(),
            visiting,
            merged,
            all_components,
            all_pages,
        )?;
    }
    visiting.remove(&canonical);

    all_components.extend(module_body.components);
    all_pages.extend(module_body.pages);
    merged.insert(canonical);

    Ok(())
}

fn resolve_import_path(
    importing_file: &Path,
    import_path: &str,
    locale: Locale,
) -> Result<PathBuf, ResolverError> {
    let base_dir = importing_file.parent().unwrap_or_else(|| Path::new("."));
    let candidate = base_dir.join(import_path);

    if !candidate.exists() {
        return Err(err_at(
            format!(
                "Imported file \"{}\" was not found (resolved from \"{}\").",
                candidate.display(),
                base_dir.display()
            ),
            importing_file,
            "",
            1,
            1,
            locale,
        ));
    }

    Ok(candidate)
}

fn canonicalize_best_effort(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_files(files: &[(&str, &str)]) -> tempfile_shim::TempDir {
        let dir = tempfile_shim::TempDir::new();
        for (name, content) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(content.as_bytes()).unwrap();
        }
        dir
    }

    mod tempfile_shim {
        use std::path::{Path, PathBuf};

        pub struct TempDir(PathBuf);
        impl TempDir {
            pub fn new() -> Self {
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                let path = std::env::temp_dir().join(format!("vibao-resolver-test-{}", nanos));
                std::fs::create_dir_all(&path).unwrap();
                TempDir(path)
            }
            pub fn path(&self) -> &Path {
                &self.0
            }
        }
        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    fn parse_root(src: &str) -> Program {
        let tokens = crate::lexer::tokenize(src).unwrap();
        crate::parser::parse(tokens).unwrap()
    }

    #[test]
    fn test_resolve_single_import_merges_component() {
        let dir = write_temp_files(&[
            (
                "app.vbao",
                r#"
                ung_dung("Test") {
                    nhap nut_bam tu "./nut_bam.vbao"
                    trang("/") { }
                }
            "#,
            ),
            (
                "nut_bam.vbao",
                r#"
                @the nut_bam(label) {
                    button(noi_dung: $label)
                }
            "#,
            ),
        ]);

        let root_path = dir.path().join("app.vbao");
        let source = std::fs::read_to_string(&root_path).unwrap();
        let program = parse_root(&source);

        let resolved = resolve(program, &root_path).unwrap();
        assert_eq!(resolved.app.components.len(), 1);
        assert_eq!(resolved.app.components[0].name, "nut_bam");
        assert!(resolved.app.imports.is_empty());
    }

    struct CwdGuard {
        original: PathBuf,
    }
    impl CwdGuard {
        fn change_to(new_dir: &Path) -> Self {
            let original = std::env::current_dir().expect("could not get the current CWD");
            std::env::set_current_dir(new_dir).expect("chdir failed");
            CwdGuard { original }
        }
    }
    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    #[test]
    fn test_resolve_with_bare_relative_path_like_real_cli_usage() {
        let dir = write_temp_files(&[
            (
                "app.vbao",
                r#"
                ung_dung("Test") {
                    nhap nut_bam tu "./nut_bam.vbao"
                    trang("/") { }
                }
            "#,
            ),
            (
                "nut_bam.vbao",
                r#"
                @the nut_bam(label) {
                    button(noi_dung: $label)
                }
            "#,
            ),
        ]);

        let _guard = CwdGuard::change_to(dir.path());

        let root_path = Path::new("app.vbao");
        let source = std::fs::read_to_string(root_path).unwrap();
        let program = parse_root(&source);
        let resolved = resolve(program, root_path).unwrap();

        assert_eq!(resolved.app.components.len(), 1);
        assert_eq!(resolved.app.components[0].name, "nut_bam");
    }

    #[test]
    fn test_resolve_missing_file_errors_clearly() {
        let dir = write_temp_files(&[(
            "app.vbao",
            r#"
                ung_dung("Test") {
                    nhap nut_bam tu "./khong_ton_tai.vbao"
                    trang("/") { }
                }
            "#,
        )]);

        let root_path = dir.path().join("app.vbao");
        let source = std::fs::read_to_string(&root_path).unwrap();
        let program = parse_root(&source);

        let result = resolve(program, &root_path);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("was not found"), "actual message: {}", msg);
    }

    #[test]
    fn test_resolve_unknown_name_in_target_file_errors_clearly() {
        let dir = write_temp_files(&[
            (
                "app.vbao",
                r#"
                ung_dung("Test") {
                    nhap ten_sai tu "./nut_bam.vbao"
                    trang("/") { }
                }
            "#,
            ),
            (
                "nut_bam.vbao",
                r#"
                @the nut_bam(label) {
                    button(noi_dung: $label)
                }
            "#,
            ),
        ]);

        let root_path = dir.path().join("app.vbao");
        let source = std::fs::read_to_string(&root_path).unwrap();
        let program = parse_root(&source);

        let result = resolve(program, &root_path);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("ten_sai"), "actual message: {}", msg);
        assert!(
            msg.contains("nut_bam"),
            "message must suggest an available name: {}",
            msg
        );
    }

    #[test]
    fn test_resolve_direct_circular_import_errors_clearly() {
        let dir = write_temp_files(&[
            (
                "a.vbao",
                r#"
                ung_dung("Test") {
                    nhap comp_b tu "./b.vbao"
                    trang("/") { }
                }
            "#,
            ),
            (
                "b.vbao",
                r#"
                nhap comp_a tu "./a.vbao"
                @the comp_b(x) {
                    button(noi_dung: $x)
                }
            "#,
            ),
        ]);

        let root_path = dir.path().join("a.vbao");
        let source = std::fs::read_to_string(&root_path).unwrap();
        let program = parse_root(&source);

        let result = resolve(program, &root_path);
        assert!(
            result.is_err(),
            "a circular import must be detected and reported"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.to_lowercase().contains("circular"),
            "actual message (not a circular import error): {}",
            msg
        );
    }

    #[test]
    fn test_resolve_diamond_import_is_not_falsely_flagged_as_circular() {
        let dir = write_temp_files(&[
            (
                "root.vbao",
                r#"
                ung_dung("Test") {
                    nhap comp_a tu "./a.vbao"
                    nhap comp_b tu "./b.vbao"
                    trang("/") { }
                }
            "#,
            ),
            (
                "a.vbao",
                r#"
                nhap shared_thing tu "./shared.vbao"
                @the comp_a(x) {
                    button(noi_dung: $x)
                }
            "#,
            ),
            (
                "b.vbao",
                r#"
                nhap shared_thing tu "./shared.vbao"
                @the comp_b(x) {
                    button(noi_dung: $x)
                }
            "#,
            ),
            (
                "shared.vbao",
                r#"
                @the shared_thing(x) {
                    text(noi_dung: $x)
                }
            "#,
            ),
        ]);

        let root_path = dir.path().join("root.vbao");
        let source = std::fs::read_to_string(&root_path).unwrap();
        let program = parse_root(&source);

        let resolved = resolve(program, &root_path).unwrap();
        let names: Vec<&str> = resolved
            .app
            .components
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(names.contains(&"comp_a"));
        assert!(names.contains(&"comp_b"));
        assert_eq!(names.iter().filter(|n| **n == "shared_thing").count(), 1);
    }
}
