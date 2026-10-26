#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct TempDir {
    pub path: PathBuf,
}

impl TempDir {
    pub fn new(label: &str) -> Self {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("vibaoc-e2e-{}-{}", label, nanos));
        fs::create_dir_all(&path).expect("failed to create temp dir");
        TempDir { path }
    }

    pub fn write(&self, name: &str, content: &str) -> PathBuf {
        let p = self.path.join(name);
        fs::write(&p, content).expect("failed to write temp file");
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn vibaoc_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_vibaoc"))
}

pub struct BuildResult {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
    pub out_dir: PathBuf,
}

impl BuildResult {
    pub fn html(&self) -> String {
        fs::read_to_string(self.out_dir.join("index.html"))
            .unwrap_or_else(|e| panic!("index.html should exist: {}", e))
    }

    pub fn js(&self) -> String {
        fs::read_to_string(self.out_dir.join("app.js"))
            .unwrap_or_else(|e| panic!("app.js should exist: {}", e))
    }

    pub fn css(&self) -> String {
        fs::read_to_string(self.out_dir.join("style.css"))
            .unwrap_or_else(|e| panic!("style.css should exist: {}", e))
    }

    pub fn assert_ok(&self) {
        assert!(
            self.ok,
            "expected `vibaoc build` to succeed, but it failed.\nstdout:\n{}\nstderr:\n{}",
            self.stdout, self.stderr
        );
    }

    pub fn assert_err(&self) {
        assert!(
            !self.ok,
            "expected `vibaoc build` to fail, but it succeeded.\nstdout:\n{}",
            self.stdout
        );
    }
}

pub fn build_source(label: &str, source: &str) -> (TempDir, BuildResult) {
    let dir = TempDir::new(label);
    let src = dir.write("app.vbao", source);
    let out_dir = dir.path.join("dist");
    let result = run_build(&src, &out_dir);
    (dir, result)
}

pub fn run_version() -> String {
    let output = Command::new(vibaoc_bin())
        .arg("--version")
        .output()
        .expect("không chạy được vibaoc --version");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

pub fn run_build(source: &PathBuf, out_dir: &PathBuf) -> BuildResult {
    run_build_with_args(source, out_dir, &[])
}

pub fn run_build_with_args(source: &PathBuf, out_dir: &PathBuf, extra: &[&str]) -> BuildResult {
    let output = Command::new(vibaoc_bin())
        .arg("build")
        .arg(source)
        .arg("--out")
        .arg(out_dir)
        .args(extra)
        .output()
        .expect("failed to spawn vibaoc binary");
    BuildResult {
        ok: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        out_dir: out_dir.clone(),
    }
}

pub fn run_check_ast(source: &PathBuf) -> (bool, String, String) {
    let output = Command::new(vibaoc_bin())
        .arg("check")
        .arg(source)
        .arg("--ast")
        .output()
        .expect("failed to spawn vibaoc binary");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}
