#[derive(Debug, Clone, Default)]
pub struct OrderedMap {
    entries: Vec<(String, String)>,
}

impl OrderedMap {
    pub fn new() -> Self {
        OrderedMap { entries: Vec::new() }
    }

    pub fn insert(&mut self, key: String, value: String) {
        if let Some(entry) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            entry.1 = value;
        } else {
            self.entries.push((key, value));
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

#[allow(dead_code)]
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn entry_or_insert_with(&mut self, key: &str, default: impl FnOnce() -> String) {
        if self.get(key).is_none() {
            self.insert(key.to_string(), default());
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}

pub fn camel_to_kebab(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for ch in s.chars() {
        if ch.is_ascii_uppercase() {
            out.push('-');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn style_map_to_string(style: &OrderedMap) -> String {
    style
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{}:{}", camel_to_kebab(k), v))
        .collect::<Vec<_>>()
        .join(";")
}

pub fn layout_css_to_string_inline(css: &crate::codegen::layout::LayoutCss) -> String {
    css.iter()
        .filter(|(_, v)| v.as_str() != "__dynamic__")
        .map(|(k, v)| format!("{}:{}", camel_to_kebab(k), v))
        .collect::<Vec<_>>()
        .join(";")
}

pub fn layout_css_to_string(selector: &str, css: &crate::codegen::layout::LayoutCss) -> String {
    let rules = css
        .iter()
        .filter(|(_, v)| v.as_str() != "__dynamic__")
        .map(|(k, v)| format!("  {}: {};", camel_to_kebab(k), v))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{} {{\n{}\n}}", selector, rules)
}

pub fn esc_attr(val: &str) -> String {
    val.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn esc_html(val: &str) -> String {
    val.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub fn indent(code: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    code.lines().map(|l| format!("{}{}", pad, l)).collect::<Vec<_>>().join("\n")
}

pub fn indent2(code: &str) -> String {
    indent(code, 2)
}

pub const BASE_CSS: &str = r#"*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: system-ui, -apple-system, sans-serif; line-height: 1.5; }
img { max-width: 100%; display: block; }
button { cursor: pointer; border: none; background: none; font: inherit; }
a { color: inherit; text-decoration: none; }
input, textarea, select { font: inherit; }
.vb-page { min-height: 100vh; }
.vb-page[data-route]:not([data-route="/"]) { display: none; }
[style*="display:none"] { display: none !important; }
[data-vb-loop] { display: none; }

@keyframes vb-fade-in { from { opacity:0 } to { opacity:1 } }
@keyframes vb-truot-len { from { opacity:0; transform:translateY(20px) } to { opacity:1; transform:translateY(0) } }
@keyframes vb-truot-xuong { from { opacity:0; transform:translateY(-20px) } to { opacity:1; transform:translateY(0) } }
@keyframes vb-phong-to { from { transform:scale(0.9); opacity:0 } to { transform:scale(1); opacity:1 } }
@keyframes vb-rung { 0%,100%{transform:translateX(0)} 25%{transform:translateX(-4px)} 75%{transform:translateX(4px)} }

.vb-anim-fade_in    { animation: vb-fade-in var(--vb-dur,0.5s) ease forwards }
.vb-anim-truot_len  { animation: vb-truot-len var(--vb-dur,0.5s) ease forwards }
.vb-anim-truot_xuong{ animation: vb-truot-xuong var(--vb-dur,0.5s) ease forwards }
.vb-anim-phong_to   { animation: vb-phong-to var(--vb-dur,0.4s) ease forwards }
.vb-anim-rung       { animation: vb-rung var(--vb-dur,0.4s) ease }

[data-vb-anim-hover] { transition: transform .2s ease, box-shadow .2s ease, filter .2s ease }
.vb-hover-phong_to  { transform: scale(1.05) !important }
.vb-hover-lam_sang  { filter: brightness(1.1) !important }
.vb-hover-nang_len  { transform: translateY(-4px) !important; box-shadow: 0 8px 24px rgba(0,0,0,.12) !important }

.vb-tabs .vb-tab-header { display:flex; gap:0; border-bottom:2px solid #e5e7eb }
.vb-tab-btn { padding:10px 20px; background:none; border:none; cursor:pointer; color:#6b7280; font-weight:500 }
.vb-tab-btn.vb-tab-active { color:#2563eb; border-bottom:2px solid #2563eb; margin-bottom:-2px }

.vb-modal-overlay { position:fixed; inset:0; background:rgba(0,0,0,.5); display:flex; align-items:center; justify-content:center; z-index:1000 }
.vb-modal-box { background:#fff; border-radius:12px; padding:24px; max-height:90vh; overflow-y:auto }

.vb-carousel { position:relative; overflow:hidden }
.vb-carousel-track { display:flex; transition:transform .3s ease }
.vb-carousel-prev,.vb-carousel-next { position:absolute; top:50%; transform:translateY(-50%); background:rgba(0,0,0,.3); color:#fff; border:none; padding:8px 14px; cursor:pointer; font-size:20px; border-radius:4px }
.vb-carousel-prev { left:8px } .vb-carousel-next { right:8px }
.vb-carousel-dots { display:flex; justify-content:center; gap:8px; padding:12px 0 }
.vb-dot { width:8px; height:8px; border-radius:50%; background:#d1d5db; border:none; cursor:pointer }
.vb-dot.vb-dot-active { background:#2563eb }

.vb-accordion-btn { width:100%; text-align:left; padding:14px 16px; background:#f9fafb; border:none; cursor:pointer; font-weight:500; display:flex; justify-content:space-between }
.vb-accordion-body { padding:16px }

.vb-dropdown { position:relative; display:inline-block }
.vb-dropdown-menu { position:absolute; top:100%; left:0; background:#fff; border:1px solid #e5e7eb; border-radius:8px; box-shadow:0 4px 20px rgba(0,0,0,.1); min-width:160px; z-index:100 }
.vb-dropdown-right { left:auto; right:0 }
.vb-dropdown-item { display:flex; align-items:center; gap:8px; width:100%; padding:10px 16px; background:none; border:none; cursor:pointer; text-align:left }
.vb-dropdown-item:hover { background:#f3f4f6 }

.vb-toast-container { position:fixed; top:16px; right:16px; display:flex; flex-direction:column; gap:8px; z-index:9999 }
.vb-toast { padding:12px 20px; border-radius:8px; color:#fff; font-weight:500; animation:vb-truot-len .3s ease }
.vb-toast-thanh_cong { background:#10b981 }
.vb-toast-loi { background:#ef4444 }
.vb-toast-canh_bao { background:#f59e0b }
.vb-toast-info { background:#3b82f6 }

.vb-form { display:flex; flex-direction:column; gap:16px }
.vb-nhom-input { display:flex; flex-direction:column; gap:6px }
.vb-nhom-input label { font-weight:500; font-size:14px; color:#374151 }
.vb-nhom-input input,.vb-nhom-input textarea,.vb-nhom-input select { padding:10px 14px; border:1px solid #d1d5db; border-radius:8px; font-size:16px; width:100% }
.vb-nhom-input input:focus,.vb-nhom-input textarea:focus { outline:none; border-color:#2563eb; box-shadow:0 0 0 3px rgba(37,99,235,.1) }
.vb-input-error { border-color:#ef4444 !important }
.vb-error-msg { color:#ef4444; font-size:13px; margin-top:4px }

.vb-spinner { border:3px solid #e5e7eb; border-top-color:#2563eb; border-radius:50%; animation:vb-spin .8s linear infinite }
@keyframes vb-spin { to { transform:rotate(360deg) } }

.vb-progress { background:#e5e7eb; border-radius:999px; overflow:hidden }
.vb-progress-bar { height:100%; background:#2563eb; transition:width .3s ease }"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_to_kebab() {
        assert_eq!(camel_to_kebab("backgroundColor"), "background-color");
        assert_eq!(camel_to_kebab("zIndex"), "z-index");
        assert_eq!(camel_to_kebab("gap"), "gap");
    }

    #[test]
    fn test_layout_css_to_string_inline_filters_dynamic_sentinel() {
        let mut css = crate::codegen::layout::LayoutCss::new();
        css.insert("width".to_string(), "__dynamic__".to_string());
        css.insert("height".to_string(), "40px".to_string());
        let result = layout_css_to_string_inline(&css);
        assert!(!result.contains("__dynamic__"), "the sentinel must not leak into static CSS: {}", result);
        assert!(result.contains("height:40px"));
    }

    #[test]
    fn test_layout_css_to_string_filters_dynamic_sentinel() {
        let mut css = crate::codegen::layout::LayoutCss::new();
        css.insert("width".to_string(), "__dynamic__".to_string());
        let result = layout_css_to_string("#vb-box-1", &css);
        assert!(!result.contains("__dynamic__"), "the sentinel must not leak into a CSS rule: {}", result);
    }

    #[test]
    fn test_ordered_map_preserves_insertion_order() {
        let mut m = OrderedMap::new();
        m.insert("z".to_string(), "1".to_string());
        m.insert("a".to_string(), "2".to_string());
        let keys: Vec<&String> = m.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["z", "a"]);
    }

    #[test]
    fn test_ordered_map_update_in_place_keeps_order() {
        let mut m = OrderedMap::new();
        m.insert("a".to_string(), "1".to_string());
        m.insert("b".to_string(), "2".to_string());
        m.insert("a".to_string(), "999".to_string());
        let entries: Vec<(&String, &String)> = m.iter().collect();
        assert_eq!(entries[0].0, "a");
        assert_eq!(entries[0].1, "999");
        assert_eq!(entries[1].0, "b");
    }

    #[test]
    fn test_ordered_map_entry_or_insert_with() {
        let mut m = OrderedMap::new();
        m.insert("borderStyle".to_string(), "dashed".to_string());
        m.entry_or_insert_with("borderStyle", || "solid".to_string());
        assert_eq!(m.get("borderStyle"), Some(&"dashed".to_string()));

        m.entry_or_insert_with("borderColor", || "black".to_string());
        assert_eq!(m.get("borderColor"), Some(&"black".to_string()));
    }

    #[test]
    fn test_style_map_to_string_skips_empty() {
        let mut m = OrderedMap::new();
        m.insert("color".to_string(), "red".to_string());
        m.insert("backgroundColor".to_string(), "".to_string());
        assert_eq!(style_map_to_string(&m), "color:red");
    }

    #[test]
    fn test_esc_attr() {
        assert_eq!(esc_attr("a\"b<c>"), "a&quot;b&lt;c&gt;");
    }

    #[test]
    fn test_esc_attr_escapes_ampersand_without_double_escaping() {
        assert_eq!(esc_attr("a & b"), "a &amp; b");
        assert_eq!(esc_attr("<a & b>"), "&lt;a &amp; b&gt;");
    }

    #[test]
    fn test_indent_adds_padding_to_each_line() {
        assert_eq!(indent("a\nb", 2), "  a\n  b");
    }
}
