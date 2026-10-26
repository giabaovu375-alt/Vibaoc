use super::child::PropsMap;
use super::expr::{ColorFuncKind, Expr};
use super::Pos;

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

#[derive(Debug, Clone, Default)]
pub struct AnimationProps {
    pub hieu_ung: Option<String>,
    pub thoi_gian: Option<u32>,
    pub tre: Option<u32>,
    pub lap: Option<LapValue>,
    pub hieu_ung_hover: Option<String>,
    pub hieu_ung_cuon: Option<String>,
}

#[derive(Debug, Clone)]
pub enum LapValue {
    Count(u32),
    MaiMai,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Breakpoint {
    DiDong,
    MayTinhBang,
    MayTinh,
}

#[derive(Debug, Clone)]
pub struct ResponsiveNode {
    pub breakpoint: Breakpoint,
    pub overrides: PropsMap,
    pub pos: Pos,
}
