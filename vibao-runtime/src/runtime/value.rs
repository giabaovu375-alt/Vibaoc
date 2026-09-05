// ============================================================
// VIBAO RUNTIME (Rust/WASM) — runtime/value.rs
// VbValue: the dynamic value type used throughout the runtime,
// equivalent to "any JS value" (string/number/bool/null/array/object)
// that __state used to hold in the old JS version.
//
// A single shared type is used (instead of a generic <T>) since the
// runtime needs to store non-uniformly-typed state within one store
// (like JS), and needs easy serialize/deserialize through
// wasm-bindgen <-> JsValue when needed for debug display or
// interacting with remaining JS code (e.g. JSON.stringify in the old
// version's __inspectState).
// ============================================================

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use wasm_bindgen::JsValue;

/// The dynamic runtime value - equivalent to "any" on the JS side.
///
/// Uses `BTreeMap` for Object instead of `HashMap` for stable iteration
/// order (important when serializing to JSON for debugging or sending
/// to an API), the same reason PropsMap in ast.rs uses a Vec instead of
/// a HashMap.
#[derive(Debug, Clone, PartialEq)]
pub enum VbValue {
    Null,
    Bool(bool),
    /// A number is always stored as f64 - matching JS Number (no
    /// int/float distinction), avoiding the need to keep 2 separate
    /// number types in sync throughout the runtime.
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
    /// Equivalent to `String(value)` on the JS side - used for
    /// bindText/toast/...
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VbValue::Null => write!(f, ""),
            VbValue::Bool(b) => write!(f, "{}", b),
            VbValue::Num(n) => {
                // JS prints an integer without ".0" (e.g. 16, not 16.0).
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
    // ── Convenience constructors ─────────────────────────────────────────

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

    // ── Type queries ────────────────────────────────────────────────

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

    /// Equivalent to `Number(value) || 0` on the JS side - used in
    /// bindProgress, lam_tron, phan_tram... where the old version always
    /// coerced to a number with a fallback of 0.
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

    /// Equivalent to JS truthiness: `!!value`. Null/false/0/""/NaN -> false.
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

    /// `.rong` (empty) - a special ViBao field on arrays/strings (see
    /// __resolveSpecialField).
    pub fn is_rong(&self) -> bool {
        match self {
            VbValue::Array(a) => a.is_empty(),
            VbValue::Str(s) => s.is_empty(),
            VbValue::Null => true,
            _ => false,
        }
    }

    /// `.do_dai` (length) - a special ViBao field: array/string length.
    pub fn do_dai(&self) -> f64 {
        match self {
            VbValue::Array(a) => a.len() as f64,
            VbValue::Str(s) => s.chars().count() as f64,
            _ => 0.0,
        }
    }

    /// Accesses a nested field via a path like "a.b.c", handling special
    /// fields (.rong/.do_dai) and numeric array indices (a path segment
    /// that's a number) itself. Equivalent to __digPath / __get in the
    /// old JS version.
    pub fn dig_path(&self, path: &str) -> VbValue {
        let mut cur = self.clone();
        for part in path.split('.') {
            if cur.is_null() {
                return VbValue::Null;
            }
            cur = cur.get_field(part);
        }
        cur
    }

    /// Gets a single field/index directly (no recursive path), checking
    /// a special field first, then an object field, then an array
    /// index.
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

    /// A JS-style "===" comparison (no type coercion) - used in
    /// bindSwitch (matching a case value) and __setState (a
    /// reference/value identity check simplified into value equality,
    /// since VbValue already Clones by value).
    pub fn strict_eq(&self, other: &VbValue) -> bool {
        self == other
    }

    /// An ordering comparison for sap_xep()/`<`/`>` - number vs number,
    /// string vs string lexicographically, and different types are
    /// treated as equal (stable, never panics).
    ///
    /// IMPORTANT NOTE for callers of this function: the
    /// `unwrap_or(Ordering::Equal)` branch is a FALLBACK for when a
    /// comparison genuinely CANNOT be made (NaN on either side, or a
    /// failed number coercion) - Equal here means "unknown / not
    /// comparable", NOT "these 2 values are equal". Gt/Lt (using `==`)
    /// are safe with this fallback (NaN will match neither Greater nor
    /// Less, matching JS semantics). BUT Gte/Lte must NOT be derived by
    /// negating the corresponding Gt/Lt (e.g. `!= Less` for Gte) - a
    /// NaN-induced Equal would slip through that negation and become
    /// `true` incorrectly. Use `is_nan_like()` below to rule this case
    /// out before composing Gte/Lte logic - see
    /// expr_eval.rs::eval_binary.
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

    /// True if an ordering comparison between these 2 values does NOT
    /// actually mean anything (the `partial_cmp_loose` result is only a
    /// fallback, not a real number/string comparison). Used so Gte/Lte
    /// never mistake "not comparable" for "equal, so >= / <= holds".
    pub fn is_nan_like(&self, other: &VbValue) -> bool {
        fn string_is_not_number(s: &str) -> bool {
            s.trim().parse::<f64>().is_err()
        }

        match (self, other) {
            (VbValue::Num(a), VbValue::Num(b)) => a.is_nan() || b.is_nan(),
            // Two strings use lexicographic comparison in partial_cmp_loose;
            // they are not numeric coercion, so this is not a NaN case.
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

    // ── JSON (debug / send to API / bindSwitch data-vb-case parsing) ─────────

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

    // ── Bridge to JS (wasm-bindgen) ───────────────────────────────────
    // Used when a value needs to be returned to the remaining JS code
    // (e.g. displaying in the devtools console, or 2-way input value
    // binding via HtmlInputElement).

    pub fn to_js_value(&self) -> JsValue {
        match self {
            VbValue::Null => JsValue::NULL,
            VbValue::Bool(b) => JsValue::from_bool(*b),
            VbValue::Num(n) => JsValue::from_f64(*n),
            VbValue::Str(s) => JsValue::from_str(s),
            VbValue::Array(_) | VbValue::Object(_) => {
                // No serde-wasm-bindgen dependency here, keeping
                // Cargo.toml lean - for Array/Object, this goes through a
                // JSON string, then JS can parse it with JSON.parse if
                // the full structure is needed. Most runtime use cases
                // (text/attr/style/input binding) display scalar values,
                // so this branch is rarely reached.
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
        assert!(VbValue::Str("0".to_string()).is_truthy()); // "0" is truthy in JS
        assert!(VbValue::Array(vec![]).is_truthy()); // [] is truthy in JS
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
