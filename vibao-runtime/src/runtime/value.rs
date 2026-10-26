use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use wasm_bindgen::JsValue;

#[derive(Debug, Clone, PartialEq)]
pub enum VbValue {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Array(Vec<VbValue>),
    Object(BTreeMap<String, VbValue>),
}

impl Default for VbValue {
    fn default() -> Self {
        VbValue::Null
    }
}

impl fmt::Display for VbValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VbValue::Null => write!(f, ""),
            VbValue::Bool(b) => write!(f, "{}", b),
            VbValue::Num(n) => {
                if n.fract() == 0.0 && n.is_finite() {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            VbValue::Str(s) => write!(f, "{}", s),
            VbValue::Array(_) | VbValue::Object(_) => {
                write!(f, "{}", self.to_json_string())
            }
        }
    }
}

impl VbValue {

    pub fn str(s: impl Into<String>) -> Self {
        VbValue::Str(s.into())
    }

    pub fn num(n: f64) -> Self {
        VbValue::Num(n)
    }

    pub fn bool(b: bool) -> Self {
        VbValue::Bool(b)
    }

    pub fn array(items: Vec<VbValue>) -> Self {
        VbValue::Array(items)
    }

    pub fn object(entries: Vec<(String, VbValue)>) -> Self {
        VbValue::Object(entries.into_iter().collect())
    }

    pub fn is_null(&self) -> bool {
        matches!(self, VbValue::Null)
    }

    pub fn is_array(&self) -> bool {
        matches!(self, VbValue::Array(_))
    }

    pub fn as_array(&self) -> Option<&Vec<VbValue>> {
        match self {
            VbValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<VbValue>> {
        match self {
            VbValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, VbValue>> {
        match self {
            VbValue::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            VbValue::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            VbValue::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn to_num_or_zero(&self) -> f64 {
        match self {
            VbValue::Num(n) => *n,
            VbValue::Str(s) => s.trim().parse().unwrap_or(0.0),
            VbValue::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            VbValue::Null => false,
            VbValue::Bool(b) => *b,
            VbValue::Num(n) => *n != 0.0 && !n.is_nan(),
            VbValue::Str(s) => !s.is_empty(),
            VbValue::Array(_) => true,
            VbValue::Object(_) => true,
        }
    }

    pub fn is_rong(&self) -> bool {
        match self {
            VbValue::Array(a) => a.is_empty(),
            VbValue::Str(s) => s.is_empty(),
            VbValue::Null => true,
            _ => false,
        }
    }

    pub fn do_dai(&self) -> f64 {
        match self {
            VbValue::Array(a) => a.len() as f64,
            VbValue::Str(s) => s.chars().count() as f64,
            _ => 0.0,
        }
    }

    pub fn dig_path(&self, path: &str) -> VbValue {
        let mut parts = path.split('.');
        let Some(first) = parts.next() else {
            return VbValue::Null;
        };
        let mut cur = self.get_field(first);
        for part in parts {
            if cur.is_null() {
                return VbValue::Null;
            }
            cur = cur.get_field(part);
        }
        cur
    }

    pub fn get_field(&self, field: &str) -> VbValue {
        match field {
            "rong" => VbValue::Bool(self.is_rong()),
            "do_dai" => VbValue::Num(self.do_dai()),
            _ => match self {
                VbValue::Object(o) => o.get(field).cloned().unwrap_or(VbValue::Null),
                VbValue::Array(a) => field
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| a.get(i))
                    .cloned()
                    .unwrap_or(VbValue::Null),
                _ => VbValue::Null,
            },
        }
    }

    pub fn strict_eq(&self, other: &VbValue) -> bool {
        self == other
    }

    pub fn partial_cmp_loose(&self, other: &VbValue) -> Ordering {
        match (self, other) {
            (VbValue::Num(a), VbValue::Num(b)) => {
                a.partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (VbValue::Str(a), VbValue::Str(b)) => a.cmp(b),
            _ => {
                let a = self.to_num_or_zero();
                let b = other.to_num_or_zero();
                a.partial_cmp(&b).unwrap_or(Ordering::Equal)
            }
        }
    }

    pub fn is_nan_like(&self, other: &VbValue) -> bool {
        fn string_is_not_number(s: &str) -> bool {
            s.trim().parse::<f64>().is_err()
        }

        match (self, other) {
            (VbValue::Num(a), VbValue::Num(b)) => a.is_nan() || b.is_nan(),
            (VbValue::Str(_), VbValue::Str(_)) => false,
            (VbValue::Str(s), VbValue::Num(n)) | (VbValue::Num(n), VbValue::Str(s)) => {
                n.is_nan() || string_is_not_number(s)
            }
            _ => {
                let a = self.to_num_or_zero();
                let b = other.to_num_or_zero();
                a.is_nan() || b.is_nan()
            }
        }
    }

    pub fn to_json_string(&self) -> String {
        match self {
            VbValue::Null => "null".to_string(),
            VbValue::Bool(b) => b.to_string(),
            VbValue::Num(n) => n.to_string(),
            VbValue::Str(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
            VbValue::Array(items) => {
                let parts: Vec<String> = items.iter().map(|v| v.to_json_string()).collect();
                format!("[{}]", parts.join(","))
            }
            VbValue::Object(map) => {
                let parts: Vec<String> = map
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}:{}",
                            serde_json::to_string(k).unwrap_or_else(|_| "\"\"".to_string()),
                            v.to_json_string()
                        )
                    })
                    .collect();
                format!("{{{}}}", parts.join(","))
            }
        }
    }

    pub fn from_json_str(s: &str) -> VbValue {
        serde_json::from_str::<serde_json::Value>(s)
            .map(VbValue::from)
            .unwrap_or(VbValue::Str(s.to_string()))
    }

    pub fn to_js_value(&self) -> JsValue {
        match self {
            VbValue::Null => JsValue::NULL,
            VbValue::Bool(b) => JsValue::from_bool(*b),
            VbValue::Num(n) => JsValue::from_f64(*n),
            VbValue::Str(s) => JsValue::from_str(s),
            VbValue::Array(_) | VbValue::Object(_) => {
                JsValue::from_str(&self.to_json_string())
            }
        }
    }

    pub fn from_js_value(v: &JsValue) -> VbValue {
        if v.is_null() || v.is_undefined() {
            return VbValue::Null;
        }
        if let Some(b) = v.as_bool() {
            return VbValue::Bool(b);
        }
        if let Some(n) = v.as_f64() {
            return VbValue::Num(n);
        }
        if let Some(s) = v.as_string() {
            return VbValue::Str(s);
        }
        VbValue::Null
    }
}

impl From<serde_json::Value> for VbValue {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => VbValue::Null,
            serde_json::Value::Bool(b) => VbValue::Bool(b),
            serde_json::Value::Number(n) => VbValue::Num(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => VbValue::Str(s),
            serde_json::Value::Array(items) => {
                VbValue::Array(items.into_iter().map(VbValue::from).collect())
            }
            serde_json::Value::Object(map) => {
                VbValue::Object(map.into_iter().map(|(k, v)| (k, VbValue::from(v))).collect())
            }
        }
    }
}

impl From<&str> for VbValue {
    fn from(s: &str) -> Self {
        VbValue::Str(s.to_string())
    }
}

impl From<String> for VbValue {
    fn from(s: String) -> Self {
        VbValue::Str(s)
    }
}

impl From<f64> for VbValue {
    fn from(n: f64) -> Self {
        VbValue::Num(n)
    }
}

impl From<bool> for VbValue {
    fn from(b: bool) -> Self {
        VbValue::Bool(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_integer_like_number() {
        assert_eq!(VbValue::Num(16.0).to_string(), "16");
        assert_eq!(VbValue::Num(16.5).to_string(), "16.5");
    }

    #[test]
    fn test_truthy_matches_js_semantics() {
        assert!(!VbValue::Null.is_truthy());
        assert!(!VbValue::Num(0.0).is_truthy());
        assert!(!VbValue::Str(String::new()).is_truthy());
        assert!(VbValue::Str("0".to_string()).is_truthy());
        assert!(VbValue::Array(vec![]).is_truthy());
    }

    #[test]
    fn test_rong_and_do_dai() {
        let arr = VbValue::Array(vec![VbValue::Num(1.0), VbValue::Num(2.0)]);
        assert!(!arr.is_rong());
        assert_eq!(arr.do_dai(), 2.0);

        let empty = VbValue::Array(vec![]);
        assert!(empty.is_rong());
        assert_eq!(empty.do_dai(), 0.0);
    }

    #[test]
    fn test_dig_path_nested_object() {
        let mut inner = BTreeMap::new();
        inner.insert("ten".to_string(), VbValue::str("An"));
        let mut outer = BTreeMap::new();
        outer.insert("nguoi_dung".to_string(), VbValue::Object(inner));
        let root = VbValue::Object(outer);

        let result = root.dig_path("nguoi_dung.ten");
        assert_eq!(result.as_str(), Some("An"));
    }

    #[test]
    fn test_dig_path_special_field_through_path() {
        let arr = VbValue::Array(vec![VbValue::Num(1.0)]);
        let mut outer = BTreeMap::new();
        outer.insert("ds".to_string(), arr);
        let root = VbValue::Object(outer);

        let result = root.dig_path("ds.do_dai");
        assert_eq!(result.as_num(), Some(1.0));
    }

    #[test]
    fn test_dig_path_single_segment() {
        let mut outer = BTreeMap::new();
        outer.insert("ten".to_string(), VbValue::str("An"));
        let root = VbValue::Object(outer);
        assert_eq!(root.dig_path("ten").as_str(), Some("An"));
    }

    #[test]
    fn test_dig_path_missing_field_returns_null() {
        let root = VbValue::Object(BTreeMap::new());
        assert_eq!(root.dig_path("khong_ton_tai"), VbValue::Null);
        assert_eq!(root.dig_path("a.b.c"), VbValue::Null);
    }

    #[test]
    fn test_json_roundtrip() {
        let v = VbValue::object(vec![
            ("a".to_string(), VbValue::num(1.0)),
            ("b".to_string(), VbValue::str("x")),
        ]);
        let json = v.to_json_string();
        let parsed = VbValue::from_json_str(&json);
        assert_eq!(parsed, v);
    }
}
