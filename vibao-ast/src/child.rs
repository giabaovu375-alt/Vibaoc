// ============================================================
// VIBAO COMPILER (Rust) — ast/child.rs
// CHILD NODE (everything nested inside a page/box/etc.) & ELEMENT
// ============================================================

use super::control_flow::{IfNode, LoopNode, SwitchNode};
use super::decl::{StateDecl, VarDecl};
use super::event::EventNode;
use super::expr::Expr;
use super::program::PageEvent;
use super::semantic::Tag;
use super::style::{AnimationProps, ResponsiveNode};
use super::Pos;

// ════════════════════════════════════════════════════════════
// 6. CHILD NODE — everything nested inside a page/box/etc.
// ════════════════════════════════════════════════════════════

/// Equivalent to the "ChildNode" union in the TS version. The Rust enum
/// wraps some variants in Box<T> because Child appears recursively
/// inside the very structs it contains (Element holds a Vec<Child>; without
/// Box, Child's size would depend recursively on itself — the Rust
/// compiler doesn't allow an infinitely-sized type like that. This is a
/// fundamental difference from TS, where everything is an implicit
/// reference and no explicit Box is needed.
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

// ════════════════════════════════════════════════════════════
// 7. ELEMENT — a concrete component (text, box, flex, button, ...)
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Element {
    /// Boundary wiring bug already fixed (settled flow: user writes "box"
    /// or "khoi" -> Lexer -> Token/surface name -> Resolver -> Tag::Khoi
    /// -> AST -> Codegen matches Tag::Khoi): this field used to be a raw
    /// String (the surface name the dev typed, in Vietnamese) — which
    /// forced the ENTIRE codegen to match on strings in hundreds of
    /// places, and made it impossible to treat "box"/"khoi" (two spellings
    /// of the SAME concept) as equivalent without manual normalization
    /// everywhere. Now `tag` is ALWAYS a Tag (the real semantic identity,
    /// see vibao_ast::semantic) — the parser guarantees this invariant
    /// (Child::Element is ONLY created once the lexer has confirmed this
    /// is one of the 40 built-in tags, via TokenKind::Component — see
    /// parser/app.rs::parse_child). The original surface name (whether the
    /// dev typed "box" or "khoi") is no longer needed after this resolve
    /// step — codegen doesn't care which language the dev wrote in, only
    /// the MEANING (Tag::Khoi) — following the principle that "locale only
    /// changes spelling, never behavior".
    pub tag: Tag,
    pub props: PropsMap,
    pub children: Vec<Child>,
    pub events: Vec<EventNode>,
    pub responsive: Vec<ResponsiveNode>,
    pub animation: AnimationProps,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct ComponentCall {
    pub name: String,
    pub props: PropsMap,
    pub children: Vec<Child>,
    pub pos: Pos,
}

/// Props map: key -> Expr. Uses Vec<(String, Expr)> instead of HashMap to
/// preserve the declaration ORDER of props as written in the source —
/// important for codegen when it needs to reproduce CSS property order or
/// produce more readable debug output. Rust's HashMap does not guarantee
/// iteration order.
pub type PropsMap = Vec<(String, Expr)>;

/// Convenience lookup for a prop by name in a PropsMap (HashMap would give
/// this for free; we have to write it by hand since we use Vec to
/// preserve order).
pub fn get_prop<'a>(props: &'a PropsMap, key: &str) -> Option<&'a Expr> {
    props.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}
