// ============================================================
// VIBAO COMPILER (Rust) — ast/program.rs
// ROOT PROGRAM & PAGE: Program, App, Page, PageEvent
// ============================================================

use super::child::Child;
use super::decl::{ComponentDef, StateDecl, Theme, VarDecl};
use super::event::Action;
use super::style::ColorValue;
use super::Pos;

// ════════════════════════════════════════════════════════════
// 2. ROOT PROGRAM
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Program {
    pub app: App,
}

#[derive(Debug, Clone)]
pub struct App {
    pub name: String,
    pub imports: Vec<ImportDecl>, // `nhap X tu "..."`
    pub variables: Vec<VarDecl>,
    pub themes: Vec<Theme>,
    pub components: Vec<ComponentDef>, // `@the`
    pub pages: Vec<Page>,
    pub pos: Pos,
}

/// Import declaration: `nhap ten_a tu "duong_dan.vbao"`, or the
/// multi-name form `nhap { ten_a, ten_b } tu "duong_dan.vbao"`. `names`
/// always holds at least one element — the parser never produces an
/// empty ImportDecl.
///
/// `path` is stored EXACTLY as written in the source (not yet resolved
/// to an absolute path) — resolving it (relative to the FILE that
/// contains the `nhap` statement, not the process's current working
/// directory) is the responsibility of the resolver module
/// (`vibaoc/src/resolver.rs`), not the parser. The parser only records
/// the syntax.
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub names: Vec<String>,
    pub path: String,
    pub pos: Pos,
}

// ════════════════════════════════════════════════════════════
// 3. PAGE
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Page {
    pub route: String,
    pub name: Option<String>,
    pub mau_nen: Option<ColorValue>,
    pub states: Vec<StateDecl>,
    pub events: Vec<PageEvent>,
    pub children: Vec<Child>,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct PageEvent {
    pub name: PageEventName,
    pub body: Vec<Action>,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PageEventName {
    OnTai,
    OnHuy,
}
