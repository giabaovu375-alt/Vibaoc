// ============================================================
// VIBAO COMPILER (Rust) — resolver.rs
//
// The module resolver for the `nhap X tu "duong_dan.vbao"` syntax.
//
// DESIGN PHILOSOPHY: the resolver runs AFTER the parser, BEFORE
// codegen - it receives a `Program` (the root file already parsed),
// finds every ImportDecl in it, reads + tokenizes + parses the
// imported files RECURSIVELY, then MERGES every ComponentDef/Page
// found directly into `program.app` (flattening). The result is still
// an ordinary `Program` - codegen doesn't need to know ANYTHING about
// whether there were multiple files, it only ever sees one already-
// merged App. This preserves a clean layer boundary: each layer
// (lexer/parser/resolver/codegen) only knows its own job.
//
// Why NOT merge inside the parser itself? Because the parser has no
// (and shouldn't have) filesystem access - the current parser tests
// run entirely in-memory on a source string, never reading disk.
// Keeping the resolver separate preserves that property.
// ============================================================

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use vibao_ast::{ComponentDef, ImportDecl, Page, Program};
use crate::diagnostic::Locale;

/// A resolver error - DELIBERATELY kept separate from `ParseError`
/// (even though both display the same way in the terminal, via
/// `[cross mark] ...`) since the resolver encounters error kinds the
/// parser has no concept of: a missing file, a circular import, an
/// import name that doesn't match anything in the target file. Merging
/// this into ParseError would force ParseError to know about the
/// filesystem - it shouldn't.
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

fn err_at(message: impl Into<String>, path: &Path, source: &str, line: usize, column: usize, locale: Locale) -> ResolverError {
    ResolverError {
        message: message.into(),
        path: path.display().to_string(),
        source: source.to_string(),
        line: line.max(1),
        column: column.max(1),
        locale,
    }
}

/// The module's single entry point: receives a Program just parsed
/// from the root file (`root_path` is that file's path, used to
/// resolve the RELATIVE paths of `nhap` statements inside it),
/// returning a Program with every import merged - ready to hand
/// directly to codegen.
#[allow(dead_code)]
pub fn resolve(program: Program, root_path: &Path) -> Result<Program, ResolverError> {
    resolve_with_locale(program, root_path, Locale::English)
}

pub fn resolve_with_locale(mut program: Program, root_path: &Path, root_locale: Locale) -> Result<Program, ResolverError> {
    // `visiting` tracks the files currently ON the current recursion
    // path (not "ever visited") - this is exactly the set needed to
    // detect a CIRCULAR import (A imports B, B imports A) WITHOUT
    // falsely flagging a valid "diamond" case (A imports C, B imports
    // C, the root imports both A and B - C gets read twice but that's
    // not a cycle, since by the time C is read the 2nd time, it's no
    // longer on the recursion path of the first read).
    let mut visiting: HashSet<PathBuf> = HashSet::new();
    let mut merged: HashSet<PathBuf> = HashSet::new();
    let root_canonical = canonicalize_best_effort(root_path);
    visiting.insert(root_canonical);

    let mut all_components: Vec<ComponentDef> = std::mem::take(&mut program.app.components);
    let mut all_pages: Vec<Page> = std::mem::take(&mut program.app.pages);

    for import in &program.app.imports {
        resolve_one_import(import, root_path, root_locale, &mut visiting, &mut merged, &mut all_components, &mut all_pages)?;
    }

    program.app.components = all_components;
    program.app.pages = all_pages;
    // Merging is complete - clears the imports list from the resulting
    // App so codegen doesn't need to know about/handle this field (it
    // only has meaning during the resolver stage).
    program.app.imports = Vec::new();

    Ok(program)
}

/// Resolves one specific `nhap` statement: reads the file, parses it,
/// checks every name in `import.names` actually exists in it, then
/// merges RECURSIVELY (the imported file can itself `nhap` another
/// file).
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

    // Checks every name listed in `nhap { a, b } tu "..."` ACTUALLY
    // exists as an @the component in the target file - if missing,
    // raises a clear error right here instead of leaving a dev to
    // figure out why component "a" doesn't work when used.
    let defined_names: HashSet<&str> =
        module_body.components.iter().map(|c| c.name.as_str()).collect();
    for wanted in &import.names {
        if !defined_names.contains(wanted.as_str()) {
            return Err(err_at(
                format!(
                    "Imported component \"{}\" was not found in \"{}\". Available components: {}",
                    wanted,
                    resolved_path.display(),
                    if defined_names.is_empty() { "(none)".to_string() } else { defined_names.iter().copied().collect::<Vec<_>>().join(", ") }
                ),
                importing_file,
                &source,
                import.pos.line,
                import.pos.column,
                parser.locale(),
            ));
        }
    }

    // Recursion: the file just read can itself nhap another file.
    // Marks it as "visiting" BEFORE recursing so a circular import is
    // caught at the right moment, then unmarks it AFTER processing
    // (backtracking) - allowing the valid "diamond" case described in
    // resolve() to still be read correctly.
    visiting.insert(canonical.clone());
    for nested_import in &module_body.imports {
        resolve_one_import(nested_import, &resolved_path, parser.locale(), visiting, merged, all_components, all_pages)?;
    }
    visiting.remove(&canonical);

    all_components.extend(module_body.components);
    all_pages.extend(module_body.pages);
    merged.insert(canonical);

    Ok(())
}

/// Resolves `import_path` (a raw string from the source, e.g.
/// "./components/nut_bam.vbao") into a real path on disk - ALWAYS
/// relative to the directory CONTAINING the file importing it
/// (`importing_file`), NEVER relative to the process's working
/// directory. This matches what any dev familiar with imports in
/// another language (JS/TS/Rust `mod`) would expect - an import must
/// not "break" just because `vibaoc build` was run from a different
/// directory.
fn resolve_import_path(importing_file: &Path, import_path: &str, locale: Locale) -> Result<PathBuf, ResolverError> {
    let base_dir = importing_file.parent().unwrap_or_else(|| Path::new("."));
    let candidate = base_dir.join(import_path);

    if !candidate.exists() {
        return Err(err_at(
            format!(
                "Imported file \"{}\" was not found (resolved from \"{}\").",
                candidate.display(), base_dir.display()
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

/// std's regular `canonicalize()` requires the file to ACTUALLY exist
/// (already checked in resolve_import_path before this function is
/// called, so this always succeeds in practice) - this function only
/// adds a safety layer: if canonicalize ever fails for some reason
/// (e.g. an unusual filesystem permission), it falls back to the
/// original path instead of panicking. Used to compare "is this the
/// same physical file" for circular import detection - important
/// since "./a.vbao" and "a.vbao" (from 2 different import sites) could
/// be the SAME file but differ as raw strings, so comparing raw
/// strings would miss the cycle.
fn canonicalize_best_effort(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: creates a temp directory + writes .vbao files for a
    /// test, returning that directory's path (auto-deleted when
    /// TempDir is dropped).
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

    /// A minimal TempDir shim - the project deliberately keeps
    /// dependencies minimal (see CONTRIBUTING.md), so no `tempfile`
    /// crate is pulled in just for tests. Uses std::env::temp_dir() +
    /// a simple time-based random name, cleaning up on Drop.
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
            ("app.vbao", r#"
                ung_dung("Test") {
                    nhap nut_bam tu "./nut_bam.vbao"
                    trang("/") { }
                }
            "#),
            ("nut_bam.vbao", r#"
                @the nut_bam(label) {
                    button(noi_dung: $label)
                }
            "#),
        ]);

        let root_path = dir.path().join("app.vbao");
        let source = std::fs::read_to_string(&root_path).unwrap();
        let program = parse_root(&source);

        let resolved = resolve(program, &root_path).unwrap();
        assert_eq!(resolved.app.components.len(), 1);
        assert_eq!(resolved.app.components[0].name, "nut_bam");
        // imports must be cleared after resolving - codegen doesn't
        // need to see this field anymore.
        assert!(resolved.app.imports.is_empty());
    }

    /// An RAII guard: restores the original CWD when dropped, even if
    /// a panic occurs partway through (e.g. a failed assert) - avoids
    /// "leaking" a changed CWD into other tests that run later in the
    /// same process. The naive approach ("restore at the end of the
    /// function") is NOT safe enough since a panic would skip past that
    /// restore line.
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
        // This test simulates EXACTLY how the real CLI calls the
        // resolver: a dev standing inside the directory containing
        // app.vbao, typing `vibaoc build app.vbao` (with NO leading
        // "./", not an absolute path) - unlike every other test in this
        // file, which always uses an absolute path
        // (dir.path().join(...)), so wouldn't catch a bug here if one
        // existed. `Path::new("app.vbao").parent()` returns `Some("")`
        // (an empty string), NOT `None` - resolve_import_path must
        // correctly handle this "empty string means the current
        // directory" case, not just the None case.
        let dir = write_temp_files(&[
            ("app.vbao", r#"
                ung_dung("Test") {
                    nhap nut_bam tu "./nut_bam.vbao"
                    trang("/") { }
                }
            "#),
            ("nut_bam.vbao", r#"
                @the nut_bam(label) {
                    button(noi_dung: $label)
                }
            "#),
        ]);

        let _guard = CwdGuard::change_to(dir.path());

        // A simple RELATIVE path, exactly as a dev would type on the real CLI.
        let root_path = Path::new("app.vbao");
        let source = std::fs::read_to_string(root_path).unwrap();
        let program = parse_root(&source);
        let resolved = resolve(program, root_path).unwrap();

        assert_eq!(resolved.app.components.len(), 1);
        assert_eq!(resolved.app.components[0].name, "nut_bam");
        // _guard is dropped at the end of the function (even if an
        // assert above panics) -> the original CWD is always restored.
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
            ("app.vbao", r#"
                ung_dung("Test") {
                    nhap ten_sai tu "./nut_bam.vbao"
                    trang("/") { }
                }
            "#),
            ("nut_bam.vbao", r#"
                @the nut_bam(label) {
                    button(noi_dung: $label)
                }
            "#),
        ]);

        let root_path = dir.path().join("app.vbao");
        let source = std::fs::read_to_string(&root_path).unwrap();
        let program = parse_root(&source);

        let result = resolve(program, &root_path);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("ten_sai"), "actual message: {}", msg);
        assert!(msg.contains("nut_bam"), "message must suggest an available name: {}", msg);
    }

    #[test]
    fn test_resolve_direct_circular_import_errors_clearly() {
        // a.vbao imports b.vbao, b.vbao imports a.vbao back -> a direct cycle.
        let dir = write_temp_files(&[
            ("a.vbao", r#"
                ung_dung("Test") {
                    nhap comp_b tu "./b.vbao"
                    trang("/") { }
                }
            "#),
            ("b.vbao", r#"
                nhap comp_a tu "./a.vbao"
                @the comp_b(x) {
                    button(noi_dung: $x)
                }
            "#),
        ]);

        let root_path = dir.path().join("a.vbao");
        let source = std::fs::read_to_string(&root_path).unwrap();
        let program = parse_root(&source);

        let result = resolve(program, &root_path);
        assert!(result.is_err(), "a circular import must be detected and reported");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.to_lowercase().contains("circular"),
            "actual message (not a circular import error): {}", msg
        );
    }

    #[test]
    fn test_resolve_diamond_import_is_not_falsely_flagged_as_circular() {
        // A VALID "diamond": the root imports both a.vbao and b.vbao,
        // and both of THEM import shared.vbao - shared.vbao is NOT part
        // of a cycle, it's just read twice from 2 independent branches.
        // This IS a case resolve() must handle CORRECTLY (no false
        // error).
        let dir = write_temp_files(&[
            ("root.vbao", r#"
                ung_dung("Test") {
                    nhap comp_a tu "./a.vbao"
                    nhap comp_b tu "./b.vbao"
                    trang("/") { }
                }
            "#),
            ("a.vbao", r#"
                nhap shared_thing tu "./shared.vbao"
                @the comp_a(x) {
                    button(noi_dung: $x)
                }
            "#),
            ("b.vbao", r#"
                nhap shared_thing tu "./shared.vbao"
                @the comp_b(x) {
                    button(noi_dung: $x)
                }
            "#),
            ("shared.vbao", r#"
                @the shared_thing(x) {
                    text(noi_dung: $x)
                }
            "#),
        ]);

        let root_path = dir.path().join("root.vbao");
        let source = std::fs::read_to_string(&root_path).unwrap();
        let program = parse_root(&source);

        let resolved = resolve(program, &root_path).unwrap();
        let names: Vec<&str> = resolved.app.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"comp_a"));
        assert!(names.contains(&"comp_b"));
        assert_eq!(names.iter().filter(|n| **n == "shared_thing").count(), 1);
    }
}
