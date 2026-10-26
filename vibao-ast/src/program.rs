use super::child::{Child, PropsMap};
use super::decl::{ComponentDef, StateDecl, Theme, VarDecl};
use super::event::Action;
use super::style::ColorValue;
use super::Pos;

#[derive(Debug, Clone)]
pub struct Program {
    pub app: App,
}

#[derive(Debug, Clone)]
pub struct App {
    pub name: String,
    pub lang: String,
    pub imports: Vec<ImportDecl>,
    pub variables: Vec<VarDecl>,
    pub themes: Vec<Theme>,
    pub global_style: PropsMap,
    pub components: Vec<ComponentDef>,
    pub pages: Vec<Page>,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub names: Vec<String>,
    pub path: String,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub route: String,
    pub name: Option<String>,
    pub description: Option<String>,
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
