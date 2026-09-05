// ============================================================
// VIBAO COMPILER (Rust) — ast/style.rs
// COLOR VALUE, ANIMATION, RESPONSIVE
// ============================================================

use super::child::PropsMap;
use super::expr::{ColorFuncKind, Expr};
use super::Pos;

// ════════════════════════════════════════════════════════════
// 12. COLOR VALUE (used specifically for explicit color props, e.g. mau_nen)
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum ColorValue {
    Hex(String),
    Name(String),
    Variable(String),
    Func {
        func: ColorFuncKind,
        args: Vec<Expr>,
    },
}

// ════════════════════════════════════════════════════════════
// 13. ANIMATION
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Default)]
pub struct AnimationProps {
    pub hieu_ung: Option<String>,
    pub thoi_gian: Option<u32>, // duration, ms
    pub tre: Option<u32>,       // delay, ms
    pub lap: Option<LapValue>,
    pub hieu_ung_hover: Option<String>,
    pub hieu_ung_cuon: Option<String>,
}

#[derive(Debug, Clone)]
pub enum LapValue {
    Count(u32),
    MaiMai, // infinite ("forever")
}

// ════════════════════════════════════════════════════════════
// 14. RESPONSIVE
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Breakpoint {
    DiDong,      // mobile
    MayTinhBang, // tablet
    MayTinh,     // desktop
}

#[derive(Debug, Clone)]
pub struct ResponsiveNode {
    pub breakpoint: Breakpoint,
    pub overrides: PropsMap,
    pub pos: Pos,
}
