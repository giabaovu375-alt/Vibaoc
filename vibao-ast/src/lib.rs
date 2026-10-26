use serde::{Deserialize, Serialize};

pub mod child;
pub mod control_flow;
pub mod decl;
pub mod event;
pub mod expr;
pub mod program;
pub mod semantic;
pub mod style;

#[cfg(test)]
mod tests;

pub use child::{get_prop, Child, ComponentCall, Element, IdentityProps, PropBlock, PropsMap};
pub use control_flow::{CaseNode, IfNode, LoopKind, LoopNode, SwitchNode};
pub use decl::{ComponentDef, DataType, ParamDef, StateDecl, Theme, VarDecl};
pub use event::{Action, EventName, EventNode};
pub use expr::{BinaryOp, ColorFuncKind, Expr, LiteralValue, TemplatePart, UnaryOp};
pub use program::{App, ImportDecl, Page, PageEvent, PageEventName, Program};
pub use semantic::{prop_spec, tag_spec, ActionName, FunctionName, PropCategory, PropKey, PropSpec, Tag, TagKind, TagSpec};
pub use style::{AnimationProps, Breakpoint, ColorValue, LapValue, ResponsiveNode};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pos {
    pub line: usize,
    pub column: usize,
}
