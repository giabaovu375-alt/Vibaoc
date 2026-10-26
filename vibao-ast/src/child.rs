use super::control_flow::{IfNode, LoopNode, SwitchNode};
use super::decl::{StateDecl, VarDecl};
use super::event::EventNode;
use super::expr::Expr;
use super::program::PageEvent;
use super::semantic::Tag;
use super::style::{AnimationProps, ResponsiveNode};
use super::Pos;

#[derive(Debug, Clone)]
pub enum Child {
    Element(Element),
    ComponentCall(ComponentCall),
    If(Box<IfNode>),
    Switch(Box<SwitchNode>),
    Loop(Box<LoopNode>),
    StateDecl(StateDecl),
    VarDecl(VarDecl),
    PageEvent(PageEvent),
}

#[derive(Debug, Clone)]
pub struct Element {
    pub tag: Tag,
    pub content: Option<Expr>,
    pub identity: IdentityProps,
    pub blocks: Vec<PropBlock>,
    pub unknown_props: PropsMap,
    pub children: Vec<Child>,
    pub events: Vec<EventNode>,
    pub responsive: Vec<ResponsiveNode>,
    pub animation: AnimationProps,
    pub pos: Pos,
}

#[derive(Debug, Clone, Default)]
pub struct IdentityProps {
    pub id: Option<Expr>,
    pub class_binding: Option<Expr>,
    pub html_tag: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PropBlock {
    Mau(PropsMap),
    KichThuoc(PropsMap),
    KhoangCach(PropsMap),
    Chu(PropsMap),
    FlexBoCuc(PropsMap),
    GridBoCuc(PropsMap),
    ViTri(PropsMap),
    Media(PropsMap),
    NhapLieu(PropsMap),
    LienKet(PropsMap),
}

#[derive(Debug, Clone)]
pub struct ComponentCall {
    pub name: String,
    pub props: PropsMap,
    pub identity: IdentityProps,
    pub children: Vec<Child>,
    pub pos: Pos,
}

pub type PropsMap = Vec<(String, Expr)>;

pub fn get_prop<'a>(props: &'a PropsMap, key: &str) -> Option<&'a Expr> {
    props.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}
