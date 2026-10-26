use crate::codegen::css::{
    camel_to_kebab, esc_attr, esc_html, indent2, layout_css_to_string, layout_css_to_string_inline,
    style_map_to_string, OrderedMap,
};
use crate::codegen::expr::{register_expr, resolve_value, ResolvedValue};
use crate::codegen::layout::{
    build_media_query, is_layout_tag_semantic, layout_unknown_keys, resolve_layout_css,
    resolve_responsive_css, tags_where_layout_prop_applies,
};
use crate::codegen::props::expand_props;
use vibao_ast::{get_prop, AnimationProps, Element, LapValue, PropsMap};

fn content_prop_name(tag: vibao_ast::Tag) -> &'static str {
    use vibao_ast::Tag;
    match tag {
        Tag::Image | Tag::Video => "nguon",
        _ => "noi_dung",
    }
}

fn flatten_element_props(node: &Element) -> PropsMap {
    use vibao_ast::PropBlock::*;
    let mut flat: PropsMap = Vec::new();
    if let Some(content) = &node.content {
        flat.push((content_prop_name(node.tag).to_string(), content.clone()));
    }
    if let Some(id) = &node.identity.id {
        flat.push(("id".to_string(), id.clone()));
    }
    if let Some(class) = &node.identity.class_binding {
        flat.push(("lop".to_string(), class.clone()));
    }
    for block in &node.blocks {
        let map: &PropsMap = match block {
            Mau(m) | KichThuoc(m) | KhoangCach(m) | Chu(m) | FlexBoCuc(m) | GridBoCuc(m)
            | ViTri(m) | Media(m) | NhapLieu(m) | LienKet(m) => m,
        };
        flat.extend(map.iter().cloned());
    }
    flat.extend(node.unknown_props.iter().cloned());
    flat
}

pub fn is_builtin_complex_semantic(tag: vibao_ast::Tag) -> bool {
    vibao_ast::semantic::tag_spec(tag).kind == vibao_ast::semantic::TagKind::Complex
}

fn tag_to_html(tag: vibao_ast::Tag) -> &'static str {
    vibao_ast::semantic::tag_spec(tag).html_tag
}

const SELF_CLOSING: [&str; 4] = ["img", "input", "hr", "br"];

fn is_self_closing(html_tag: &str) -> bool {
    SELF_CLOSING.contains(&html_tag)
}

pub trait ElementCodegenHost {
    fn next_id(&mut self, tag: &str) -> String;
    fn gen_children(&mut self, children: &[vibao_ast::Child]) -> String;
    fn add_css(&mut self, code: &str);
    fn add_media_query(&mut self, code: &str);
    fn add_warning(&mut self, _msg: String) {}

    fn add_hoisted_template(&mut self, _html: String) {}

    fn compile_event_handler_registry(&self, event: &vibao_ast::EventNode) -> (String, String) {
        crate::codegen::action::compile_event_handler_registry(event)
    }
}

pub fn gen_element(node: &Element, host: &mut dyn ElementCodegenHost) -> String {
    let tag_name: &str = crate::locale::vi::tag_display_name_vi(node.tag);
    let auto_id = host.next_id(tag_name);
    let flat_props = flatten_element_props(node);
    let id = match get_prop(&flat_props, "id") {
        Some(expr) => match resolve_value(expr) {
            ResolvedValue::Static(s) => esc_attr(&s),
            _ => auto_id,
        },
        None => auto_id,
    };
    if is_builtin_complex_semantic(node.tag) {
        gen_complex_component(node, &id)
    } else if is_layout_tag_semantic(node.tag) {
        gen_layout_element(node, &id, host)
    } else {
        gen_simple_element(node, &id, host)
    }
}

fn gen_simple_element(node: &Element, id: &str, host: &mut dyn ElementCodegenHost) -> String {
    let tag_name: &str = crate::locale::vi::tag_display_name_vi(node.tag);
    let flat_props = flatten_element_props(node);
    let mut expanded = expand_props(node.tag, &flat_props);
    if node.tag == vibao_ast::Tag::HopKiem && !expanded.attrs.contains_key("type") {
        expanded
            .attrs
            .insert("type".to_string(), "checkbox".to_string());
    } else if node.tag == vibao_ast::Tag::ChonMot && !expanded.attrs.contains_key("type") {
        expanded
            .attrs
            .insert("type".to_string(), "radio".to_string());
    }
    for w in &expanded.warnings {
        host.add_warning(format!("tag '{}': {}", tag_name, w));
    }
    for key in &expanded.unknown_keys {
        if !(key.starts_with("data_") || key.starts_with("aria_")) {
            match crate::locale::resolve_prop_key(key) {
                Some(prop_key) => {
                    host.add_warning(format!(
                        "prop '{}' là cú pháp hợp lệ của ViBao (nhóm: {}) nhưng KHÔNG áp dụng cho thẻ '{}' \
                         — giá trị vẫn được ghi thẳng thành HTML attribute '{}=\"...\"' (không tự động \
                         chuyển kebab-case), nên nếu đây không phải gõ sai, có thể bỏ qua cảnh báo này.",
                        key, prop_key.category().display_name_vi(), tag_name, key,
                    ));
                }
                None => {
                    host.add_warning(format!(
                        "prop '{}' on tag '{}' is not recognized by ViBao - possibly a mistyped prop name. \
                         An unrecognized prop is still written directly as the HTML attribute '{}=\"...\"' (not \
                         automatically converted to kebab-case), so if this is meant to be a custom attribute, \
                         check that the name matches the intended HTML attribute format.",
                        key, tag_name, key,
                    ));
                }
            }
        }
    }
    let style_str = style_map_to_string(&expanded.style);
    let attrs_str = attrs_to_string_ordered(&expanded.attrs, &["noi_dung"]);
    let dynamic_attrs = expanded
        .dynamic
        .iter()
        .filter(|(k, _)| k.as_str() != "noi_dung")
        .map(|(k, v)| format!("data-vb-style-{}=\"{}\"", camel_to_kebab(k), esc_attr(v)))
        .collect::<Vec<_>>()
        .join(" ");
    let dynamic_attr_bindings = expanded
        .dynamic_attrs
        .iter()
        .map(|(k, v)| format!("data-vb-attr-{}=\"{}\"", k, esc_attr(v)))
        .collect::<Vec<_>>()
        .join(" ");
    let model_attr = expanded
        .model
        .as_deref()
        .map(|key| format!("data-vb-model=\"{}\"", esc_attr(key)))
        .unwrap_or_default();
    let class_binding_attr = if expanded.class_bindings.is_empty() {
        String::new()
    } else {
        format!(
            "data-vb-class=\"{}\"",
            esc_attr(&expanded.class_bindings.join(","))
        )
    };
    let anim_attrs = gen_anim_attrs(&node.animation);

    let event_attrs = node
        .events
        .iter()
        .map(|e| {
            let (attr_name, action_id) = host.compile_event_handler_registry(e);
            format!("{}=\"{}\"", attr_name, action_id)
        })
        .collect::<Vec<_>>()
        .join(" ");

    if !node.responsive.is_empty() {
        for key in crate::codegen::layout::responsive_unknown_keys(&node.responsive) {
            if let Some(msg) = responsive_unknown_key_warning(&key, tag_name) {
                host.add_warning(msg);
            }
        }
        let bp_css = resolve_responsive_css(tag_name, &node.responsive);
        for bp in &bp_css {
            let mq = build_media_query(&format!("#{}", id), bp);
            if !mq.is_empty() {
                host.add_media_query(&mq);
            }
        }
    }

    let content = if node.tag == vibao_ast::Tag::LuaChon {
        gen_select_options(&node.children)
    } else {
        get_content(tag_name, &flat_props, &node.children, host)
    };
    let default_html_tag = tag_to_html(node.tag);
    let html_tag = match node.identity.html_tag.as_deref() {
        Some(custom) if default_html_tag == "div" => custom,
        Some(custom) => {
            host.add_warning(format!(
                "tag '{}': the_html \"{}\" was ignored, only container tags (rendered as <div>) can change their HTML element",
                tag_name, custom
            ));
            default_html_tag
        }
        None => default_html_tag,
    };
    let self_closing = is_self_closing(html_tag);

    let all_attrs = [
        format!("id=\"{}\"", id),
        if style_str.is_empty() {
            String::new()
        } else {
            format!("style=\"{}\"", style_str)
        },
        attrs_str,
        dynamic_attrs,
        dynamic_attr_bindings,
        model_attr,
        class_binding_attr,
        anim_attrs,
        event_attrs,
        if !node.events.is_empty() {
            "data-vb-interactive".to_string()
        } else {
            String::new()
        },
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(" ");

    if self_closing {
        format!("<{} {} />", html_tag, all_attrs)
    } else {
        format!("<{} {}>{}</{}>", html_tag, all_attrs, content, html_tag)
    }
}

fn get_content(
    tag: &str,
    props: &vibao_ast::PropsMap,
    children: &[vibao_ast::Child],
    host: &mut dyn ElementCodegenHost,
) -> String {
    let _ = tag;
    if let Some(content_expr) = get_prop(props, "noi_dung") {
        let resolved = resolve_value(content_expr);
        return match resolved {
            ResolvedValue::Dynamic => {
                let expr_id = register_expr(content_expr.clone());
                format!("<span data-vb-text=\"{}\"></span>", expr_id)
            }
            ResolvedValue::Static(s) => esc_html(&s),
            _ => String::new(),
        };
    }
    if !children.is_empty() {
        format!("\n{}\n", indent2(&host.gen_children(children)))
    } else {
        String::new()
    }
}

fn gen_select_options(children: &[vibao_ast::Child]) -> String {
    let mut out = String::new();
    for child in children {
        let vibao_ast::Child::Element(el) = child else {
            continue;
        };
        let el_flat_props = flatten_element_props(el);
        let label = match get_prop(&el_flat_props, "noi_dung") {
            Some(expr) => match resolve_value(expr) {
                ResolvedValue::Static(s) => s,
                ResolvedValue::Dynamic => String::new(),
                other => other.as_css(),
            },
            None => String::new(),
        };
        let value = match get_prop(&el_flat_props, "gia_tri") {
            Some(expr) => match resolve_value(expr) {
                ResolvedValue::Static(s) => s,
                ResolvedValue::Dynamic => label.clone(),
                other => other.as_css(),
            },
            None => label.clone(),
        };
        out.push_str(&format!(
            "<option value=\"{}\">{}</option>\n",
            esc_attr(&value),
            esc_html(&label)
        ));
    }
    if out.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", indent2(&out))
    }
}

fn attrs_to_string_ordered(attrs: &OrderedMap, skip: &[&str]) -> String {
    attrs
        .iter()
        .filter(|(k, _)| !skip.contains(&k.as_str()))
        .map(|(k, v)| format!("{}=\"{}\"", k, esc_attr(v)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn gen_layout_element(node: &Element, id: &str, host: &mut dyn ElementCodegenHost) -> String {
    let tag_name: &str = crate::locale::vi::tag_display_name_vi(node.tag);
    let flat_props = flatten_element_props(node);
    let layout_css = resolve_layout_css(node.tag, &flat_props);
    let style_str = layout_css_to_string_inline(&layout_css);

    for key in layout_unknown_keys(node.tag, &flat_props) {
        if !(key.starts_with("data_") || key.starts_with("aria_")) {
            match crate::locale::resolve_prop_key(&key) {
                Some(prop_key) => {
                    let applicable = tags_where_layout_prop_applies(prop_key);
                    let suggestion = if applicable.is_empty() {
                        "không có thẻ layout nào hỗ trợ prop này".to_string()
                    } else {
                        let names: Vec<&str> = applicable
                            .iter()
                            .map(|t| crate::locale::vi::tag_display_name_vi(*t))
                            .collect();
                        format!("chỉ dùng được trên: {}", names.join(", "))
                    };
                    host.add_warning(format!(
                        "prop '{}' là cú pháp hợp lệ nhưng KHÔNG có tác dụng trên thẻ layout '{}' — {}. \
                         Xem docs/LIMITATIONS.md để biết giới hạn của từng loại layout.",
                        key, tag_name, suggestion
                    ));
                }
                None => {
                    host.add_warning(format!(
                        "prop '{}' trên thẻ layout '{}' không được ViBao nhận diện — có thể do gõ sai tên prop. \
                         Prop lạ trên layout element KHÔNG được ghi ra CSS/attr nào cả (khác simple element).",
                        key, tag_name
                    ));
                }
            }
        }
    }

    const DYNAMIC_LAYOUT_PROPS: [(&str, &str); 16] = [
        ("background-color", "mau_nen"),
        ("width", "rong"),
        ("height", "cao"),
        ("padding", "dem"),
        ("border-radius", "radius"),
        ("margin", "le"),
        ("box-shadow", "bong"),
        ("overflow", "cuon_tran"),
        ("z-index", "tang_z"),
        ("gap", "gap"),
        ("row-gap", "gap_doc"),
        ("column-gap", "gap_ngang"),
        ("min-width", "min_rong"),
        ("max-width", "max_rong"),
        ("min-height", "min_cao"),
        ("max-height", "max_cao"),
    ];
    let dynamic_layout_attrs = DYNAMIC_LAYOUT_PROPS
        .iter()
        .filter_map(
            |(css_prop_name, prop_key)| match get_prop(&flat_props, prop_key) {
                Some(expr @ vibao_ast::Expr::Variable(_, _)) => {
                    let expr_id = register_expr(expr.clone());
                    Some(format!("data-vb-style-{}=\"{}\"", css_prop_name, expr_id))
                }
                _ => None,
            },
        )
        .collect::<Vec<_>>()
        .join(" ");
    const UNSUPPORTED_DYNAMIC_LAYOUT_PROPS: [&str; 5] = ["huong", "vien", "can", "doc", "boc"];
    for prop_key in UNSUPPORTED_DYNAMIC_LAYOUT_PROPS {
        if let Some(vibao_ast::Expr::Variable(_, _)) = get_prop(&flat_props, prop_key) {
            host.add_warning(format!(
                "thẻ '{}': prop '{}' với giá trị biểu thức động chưa được hỗ trợ trên layout tag \
                 (cần map giá trị hoặc ghép nhiều CSS property cùng lúc, không thể bind qua 1 \
                 data-vb-style- đơn). Style sẽ KHÔNG được áp dụng. Dùng giá trị tĩnh, hoặc bọc \
                 trong 'neu/khong_thi' ở cấp element để chọn hẳn 1 nhánh lúc build.",
                tag_name, prop_key,
            ));
        }
    }
    let anim_attrs = gen_anim_attrs(&node.animation);

    host.add_css(&layout_css_to_string(&format!("#{}", id), &layout_css));

    if !node.responsive.is_empty() {
        for key in crate::codegen::layout::responsive_unknown_keys(&node.responsive) {
            if let Some(msg) = responsive_unknown_key_warning(&key, tag_name) {
                host.add_warning(msg);
            }
        }
        let bp_css = resolve_responsive_css(tag_name, &node.responsive);
        for bp in &bp_css {
            let mq = build_media_query(&format!("#{}", id), bp);
            if !mq.is_empty() {
                host.add_media_query(&mq);
            }
        }
    }

    let children_html = if node.tag == vibao_ast::Tag::Stack {
        node.children
            .iter()
            .map(|c| {
                let html = host.gen_children(std::slice::from_ref(c));
                if html.is_empty() {
                    String::new()
                } else {
                    wrap_stack_child(&html)
                }
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        host.gen_children(&node.children)
    };

    let extra_style = if node.tag == vibao_ast::Tag::Lop {
        "position:relative;"
    } else {
        ""
    };

    let html_tag = node.identity.html_tag.as_deref().unwrap_or("div");

    format!(
        "<{tag} id=\"{}\" style=\"{}{}\" {} {}>\n{}\n</{tag}>",
        id,
        extra_style,
        style_str,
        dynamic_layout_attrs,
        anim_attrs,
        indent2(&children_html),
        tag = html_tag
    )
}

fn wrap_stack_child(html: &str) -> String {
    format!("<div style=\"grid-area:1/1/2/2\">{}</div>", html)
}

fn gen_complex_component(node: &Element, id: &str) -> String {
    let tag_name: &str = crate::locale::vi::tag_display_name_vi(node.tag);
    format!(
        "<div id=\"{}\" data-vb-component=\"{}\"><!-- {} --></div>",
        id, tag_name, tag_name
    )
}

pub(crate) fn tag_to_class_name(tag: &str) -> &str {
    match tag {
        "khoi" => "box",
        "cuon" => "scroll",
        "lop" => "layer",
        "khoang_cach" => "spacer",
        "duong_ke" => "divider",
        "can_giua" => "container",
        "dinh_dau" => "sticky",
        "dinh_man_hinh" => "fixed",
        other => other,
    }
}

#[cfg(test)]
pub fn make_class_name(tag: &str, index: u32) -> String {
    format!("vb-{}-{}", tag_to_class_name(tag), index)
}

pub fn gen_anim_attrs(anim: &AnimationProps) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(hieu_ung) = &anim.hieu_ung {
        parts.push(format!("data-vb-anim=\"{}\"", hieu_ung));
        if let Some(ms) = anim.thoi_gian {
            parts.push(format!("data-vb-anim-duration=\"{}\"", ms));
        }
        if let Some(ms) = anim.tre {
            parts.push(format!("data-vb-anim-delay=\"{}\"", ms));
        }
        if let Some(lap) = &anim.lap {
            let lap_str = match lap {
                LapValue::Count(n) => n.to_string(),
                LapValue::MaiMai => "infinite".to_string(),
            };
            parts.push(format!("data-vb-anim-repeat=\"{}\"", lap_str));
        }
    }

    if let Some(hover) = &anim.hieu_ung_hover {
        let dur = anim.thoi_gian.unwrap_or(300);
        parts.push(format!("data-vb-anim-hover=\"{}:{}\"", hover, dur));
    }

    if let Some(scroll) = &anim.hieu_ung_cuon {
        let dur = anim.thoi_gian.unwrap_or(600);
        let delay = anim.tre.unwrap_or(0);
        parts.push(format!(
            "data-vb-anim-scroll=\"{}:{}:{}\"",
            scroll, dur, delay
        ));
    }

    parts.join(" ")
}

fn responsive_unknown_key_warning(key: &str, tag_name: &str) -> Option<String> {
    if key.starts_with("data_") || key.starts_with("aria_") {
        return None;
    }
    let msg = if crate::locale::resolve_prop_key(key).is_some() {
        format!(
            "prop '{}' trong khối responsive (@di_dong/@may_tinh_bang/@may_tinh) trên thẻ '{}' \
             LÀ 1 prop hợp lệ của ViBao, nhưng CHƯA được hỗ trợ trực tiếp trong ngữ cảnh \
             responsive (chỉ 7 prop được xử lý riêng ở đây: cot/huong/co/rong/cao/dem/an — \
             xem RESPONSIVE_HANDLED_PROPS). Giá trị vẫn được chuyển thành 1 CSS property tuỳ ý \
             qua passthrough (camelCase->kebab-case) nên có thể hoạt động tình cờ đúng, nhưng \
             KHÔNG được đảm bảo — không phải gõ sai, có thể yên tâm bỏ qua nếu kết quả đang \
             hiển thị đúng ý muốn.",
            key, tag_name
        )
    } else {
        format!(
            "prop '{}' trong khối responsive (@di_dong/@may_tinh_bang/@may_tinh) trên thẻ '{}' \
             không được ViBao nhận diện — có thể do gõ sai tên prop. Giá trị VẪN được chuyển \
             thành 1 CSS property tuỳ ý (qua camelCase->kebab-case, hành vi passthrough giữ \
             nguyên như cũ để không phá vỡ các trường hợp cố ý dùng CSS property tuỳ ý), nên \
             nếu đây không phải gõ sai, có thể bỏ qua cảnh báo này.",
            key, tag_name
        )
    };
    Some(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Breakpoint, Child, Element, Expr, LoopKind, LoopNode, Pos, ResponsiveNode};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    struct FakeHost {
        counter: u32,
        js: Vec<String>,
        css: Vec<String>,
        media: Vec<String>,
        warnings: Vec<String>,
        hoisted_templates: Vec<String>,
    }

    impl FakeHost {
        fn new() -> Self {
            FakeHost {
                counter: 0,
                js: vec![],
                css: vec![],
                media: vec![],
                warnings: vec![],
                hoisted_templates: vec![],
            }
        }
    }

    impl ElementCodegenHost for FakeHost {
        fn next_id(&mut self, tag: &str) -> String {
            self.counter += 1;
            format!("vb-{}-{}", tag, self.counter)
        }
        fn gen_children(&mut self, children: &[Child]) -> String {
            children
                .iter()
                .map(|c| match c {
                    Child::Element(el) => gen_element(el, self),
                    Child::Loop(node) => crate::codegen::control::gen_loop(node, self),
                    Child::If(node) => crate::codegen::control::gen_if(node, self),
                    Child::Switch(node) => crate::codegen::control::gen_switch(node, self),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        fn add_css(&mut self, code: &str) {
            self.css.push(code.to_string());
        }
        fn add_media_query(&mut self, code: &str) {
            self.media.push(code.to_string());
        }
        fn add_warning(&mut self, msg: String) {
            self.warnings.push(msg);
        }
        fn add_hoisted_template(&mut self, html: String) {
            self.hoisted_templates.push(html);
        }
    }

    fn make_element(tag: vibao_ast::Tag, props: vibao_ast::PropsMap) -> Element {
        Element {
            tag,
            content: None,
            identity: Default::default(),
            blocks: vec![],
            unknown_props: props,
            children: vec![],
            events: vec![],
            responsive: vec![],
            animation: AnimationProps::default(),
            pos: p(),
        }
    }

    #[test]
    fn test_static_text_is_html_escaped() {
        let props: vibao_ast::PropsMap = vec![(
            "noi_dung".to_string(),
            Expr::Literal(
                vibao_ast::LiteralValue::Str("A < B & C > D".to_string()),
                p(),
            ),
        )];
        let el = make_element(vibao_ast::Tag::Text, props);
        let mut host = FakeHost::new();
        let html = gen_element(&el, &mut host);
        assert!(html.contains("A &lt; B &amp; C &gt; D"));
        assert!(!html.contains("A < B & C > D"));
    }

    #[test]
    fn test_static_attributes_are_html_attribute_escaped() {
        let mut attrs = OrderedMap::new();
        attrs.insert("alt".to_string(), "Ảnh \"đẹp\" <test>".to_string());
        let html = attrs_to_string_ordered(&attrs, &[]);
        assert_eq!(html, "alt=\"Ảnh &quot;đẹp&quot; &lt;test&gt;\"");
    }

    #[test]
    fn test_gen_simple_element_warns_on_unknown_prop() {
        let props: vibao_ast::PropsMap = vec![(
            "mua".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("do".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Text, props);
        let mut host = FakeHost::new();
        gen_element(&el, &mut host);
        assert_eq!(host.warnings.len(), 1);
        assert!(host.warnings[0].contains("mua"));
        assert!(host.warnings[0].contains("text"));
    }

    #[test]
    fn test_gen_simple_element_warns_with_category_for_valid_layout_only_prop() {
        let props: vibao_ast::PropsMap = vec![("cot".to_string(), Expr::literal_str("2", p()))];
        let el = make_element(vibao_ast::Tag::Text, props);
        let mut host = FakeHost::new();
        gen_element(&el, &mut host);
        assert_eq!(host.warnings.len(), 1);
        assert!(
            host.warnings[0].contains("hợp lệ"),
            "phải nói rõ đây là prop hợp lệ: {}",
            host.warnings[0]
        );
        assert!(
            !host.warnings[0].contains("mistyped"),
            "không được lẫn message gõ sai tiếng Anh cũ: {}",
            host.warnings[0]
        );
    }

    #[test]
    fn test_gen_simple_element_no_warning_for_data_prefixed_prop() {
        let props: vibao_ast::PropsMap = vec![(
            "data_testid".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("hero".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Text, props);
        let mut host = FakeHost::new();
        gen_element(&el, &mut host);
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn test_gen_simple_element_no_warning_for_known_prop() {
        let props: vibao_ast::PropsMap = vec![(
            "mau".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Color("#FF0000".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Text, props);
        let mut host = FakeHost::new();
        gen_element(&el, &mut host);
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn test_tag_to_html_mapping() {
        use vibao_ast::Tag;
        assert_eq!(tag_to_html(Tag::Text), "p");
        assert_eq!(tag_to_html(Tag::Link), "a");
        assert_eq!(tag_to_html(Tag::Image), "img");
        assert_eq!(tag_to_html(Tag::ThanhDieuHuong), "nav");
        assert_eq!(tag_to_html(Tag::HopKiem), "input");
        assert_eq!(tag_to_html(Tag::ChonMot), "input");
        assert_eq!(tag_to_html(Tag::LuaChon), "select");
        assert_eq!(tag_to_html(Tag::Textarea), "textarea");
        assert_eq!(tag_to_html(Tag::Khoi), "div");
    }

    #[test]
    fn test_checkbox_defaults_to_type_checkbox() {
        let el = make_element(vibao_ast::Tag::HopKiem, vec![]);
        let mut host = FakeHost::new();
        let html = gen_simple_element(&el, "vb-hop-kiem-1", &mut host);
        assert!(html.starts_with("<input "));
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.ends_with("/>"));
    }

    #[test]
    fn test_checkbox_respects_explicit_type_override() {
        let el = make_element(
            vibao_ast::Tag::HopKiem,
            vec![("loai".to_string(), Expr::literal_str("checkbox", p()))],
        );
        let mut host = FakeHost::new();
        let html = gen_simple_element(&el, "vb-hop-kiem-2", &mut host);
        assert_eq!(html.matches("type=\"checkbox\"").count(), 1);
    }

    #[test]
    fn test_checkbox_with_gia_tri_variable_generates_model_attr() {
        let el = make_element(
            vibao_ast::Tag::HopKiem,
            vec![(
                "gia_tri".to_string(),
                Expr::Variable("dong_y".to_string(), p()),
            )],
        );
        let mut host = FakeHost::new();
        let html = gen_simple_element(&el, "vb-hop-kiem-3", &mut host);
        assert!(html.contains("data-vb-model=\"dong_y\""));
        assert!(!html.contains("data-vb-attr-value"));
    }

    #[test]
    fn test_radio_defaults_to_type_radio() {
        let el = make_element(vibao_ast::Tag::ChonMot, vec![]);
        let mut host = FakeHost::new();
        let html = gen_simple_element(&el, "vb-chon-mot-1", &mut host);
        assert!(html.contains("type=\"radio\""));
    }

    #[test]
    fn test_radio_group_combines_gia_tri_model_and_chon_static_value() {
        let el = make_element(
            vibao_ast::Tag::ChonMot,
            vec![
                (
                    "gia_tri".to_string(),
                    Expr::Variable("chu_de".to_string(), p()),
                ),
                ("chon".to_string(), Expr::literal_str("sang", p())),
            ],
        );
        let mut host = FakeHost::new();
        let html = gen_simple_element(&el, "vb-chon-mot-1", &mut host);
        assert!(html.contains("data-vb-model=\"chu_de\""));
        assert!(html.contains("value=\"sang\""));
    }

    #[test]
    fn test_nav_renders_as_nav_tag_with_children() {
        let mut host = FakeHost::new();
        let inner = make_element(
            vibao_ast::Tag::Text,
            vec![("noi_dung".to_string(), Expr::literal_str("Home", p()))],
        );
        let el = Element {
            tag: vibao_ast::Tag::ThanhDieuHuong,
            content: None,
            identity: Default::default(),
            blocks: vec![],
            unknown_props: vec![],
            children: vec![vibao_ast::Child::Element(inner)],
            events: vec![],
            responsive: vec![],
            animation: AnimationProps::default(),
            pos: p(),
        };
        let html = gen_element(&el, &mut host);
        assert!(html.starts_with("<nav "));
        assert!(html.contains("</nav>"));
    }

    #[test]
    fn test_form_renders_as_form_tag_with_children_not_placeholder() {
        let mut host = FakeHost::new();
        let input = make_element(
            vibao_ast::Tag::Input,
            vec![("loai".to_string(), Expr::literal_str("email", p()))],
        );
        let el = Element {
            tag: vibao_ast::Tag::Form,
            content: None,
            identity: Default::default(),
            blocks: vec![],
            unknown_props: vec![],
            children: vec![vibao_ast::Child::Element(input)],
            events: vec![],
            responsive: vec![],
            animation: AnimationProps::default(),
            pos: p(),
        };
        let html = gen_element(&el, &mut host);
        assert!(html.starts_with("<form "));
        assert!(html.contains("</form>"));
        assert!(!html.contains("data-vb-component"));
        assert!(html.contains("<input "));
    }

    fn element_with_html_tag(tag: vibao_ast::Tag, html_tag: &str) -> Element {
        Element {
            tag,
            content: None,
            identity: vibao_ast::IdentityProps {
                id: None,
                class_binding: None,
                html_tag: Some(html_tag.to_string()),
            },
            blocks: vec![],
            unknown_props: vec![],
            children: vec![],
            events: vec![],
            responsive: vec![],
            animation: AnimationProps::default(),
            pos: p(),
        }
    }

    #[test]
    fn test_html_tag_override_replaces_div_on_a_container() {
        let mut host = FakeHost::new();
        let html = gen_element(
            &element_with_html_tag(vibao_ast::Tag::Khoi, "section"),
            &mut host,
        );
        assert!(html.starts_with("<section "), "{}", html);
        assert!(html.ends_with("</section>"), "{}", html);
        assert!(!html.contains("<div"), "{}", html);
    }

    #[test]
    fn test_html_tag_override_is_ignored_with_a_warning_on_non_container_tags() {
        let mut host = FakeHost::new();
        let html = gen_element(
            &element_with_html_tag(vibao_ast::Tag::Button, "section"),
            &mut host,
        );
        assert!(html.starts_with("<button "), "{}", html);
        assert!(
            host.warnings.iter().any(|w| w.contains("the_html")),
            "{:?}",
            host.warnings
        );
    }

    #[test]
    fn test_custom_id_overrides_auto_generated_id_on_simple_tag() {
        let el = make_element(
            vibao_ast::Tag::Text,
            vec![
                ("noi_dung".to_string(), Expr::literal_str("Xin chao", p())),
                ("id".to_string(), Expr::literal_str("gioi-thieu", p())),
            ],
        );
        let mut host = FakeHost::new();
        let html = gen_element(&el, &mut host);
        assert!(html.contains("id=\"gioi-thieu\""));
        assert_eq!(html.matches("id=\"").count(), 1);
    }

    #[test]
    fn test_custom_id_overrides_auto_generated_id_on_layout_tag() {
        let mut el = make_element(vibao_ast::Tag::Khoi, vec![]);
        el.identity.id = Some(Expr::literal_str("du-an", p()));
        let mut host = FakeHost::new();
        let html = gen_element(&el, &mut host);
        assert!(html.contains("id=\"du-an\""));
        assert_eq!(html.matches("id=\"").count(), 1);
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn test_custom_id_is_attribute_escaped() {
        let mut el = make_element(vibao_ast::Tag::Khoi, vec![]);
        el.identity.id = Some(Expr::literal_str("a\"b&c", p()));
        let mut host = FakeHost::new();
        let html = gen_element(&el, &mut host);
        assert!(html.contains("id=\"a&quot;b&amp;c\""), "{}", html);
    }

    #[test]
    fn test_no_custom_id_keeps_auto_generated_id() {
        let el = make_element(vibao_ast::Tag::Khoi, vec![]);
        let mut host = FakeHost::new();
        let html = gen_element(&el, &mut host);
        assert!(html.contains("id=\"vb-khoi-"));
    }

    #[test]
    fn test_select_children_become_options() {
        let opt_a = make_element(
            vibao_ast::Tag::Text,
            vec![
                ("noi_dung".to_string(), Expr::literal_str("Táo", p())),
                ("gia_tri".to_string(), Expr::literal_str("apple", p())),
            ],
        );
        let opt_b = make_element(
            vibao_ast::Tag::Text,
            vec![("noi_dung".to_string(), Expr::literal_str("Cam", p()))],
        );
        let el = Element {
            tag: vibao_ast::Tag::LuaChon,
            content: None,
            identity: Default::default(),
            blocks: vec![],
            unknown_props: vec![],
            children: vec![
                vibao_ast::Child::Element(opt_a),
                vibao_ast::Child::Element(opt_b),
            ],
            events: vec![],
            responsive: vec![],
            animation: AnimationProps::default(),
            pos: p(),
        };
        let mut host = FakeHost::new();
        let html = gen_element(&el, &mut host);
        assert!(html.starts_with("<select "));
        assert!(html.contains("<option value=\"apple\">Táo</option>"));
        assert!(html.contains("<option value=\"Cam\">Cam</option>"));
    }

    #[test]
    fn test_select_with_gia_tri_variable_generates_model_attr() {
        let el = make_element(
            vibao_ast::Tag::LuaChon,
            vec![(
                "gia_tri".to_string(),
                Expr::Variable("mau".to_string(), p()),
            )],
        );
        let mut host = FakeHost::new();
        let html = gen_element(&el, &mut host);
        assert!(html.contains("data-vb-model=\"mau\""));
        assert!(!html.contains(" value=\""));
    }

    #[test]
    fn test_textarea_renders_as_textarea_tag() {
        let el = make_element(vibao_ast::Tag::Textarea, vec![]);
        let mut host = FakeHost::new();
        let html = gen_element(&el, &mut host);
        assert!(html.starts_with("<textarea "));
        assert!(html.contains("</textarea>"));
    }

    #[test]
    fn test_textarea_with_gia_tri_variable_generates_model_attr() {
        let el = make_element(
            vibao_ast::Tag::Textarea,
            vec![(
                "gia_tri".to_string(),
                Expr::Variable("mo_ta".to_string(), p()),
            )],
        );
        let mut host = FakeHost::new();
        let html = gen_element(&el, &mut host);
        assert!(html.contains("data-vb-model=\"mo_ta\""));
        assert!(!html.contains(" value=\""));
    }

    #[test]
    fn test_self_closing_tags() {
        assert!(is_self_closing("img"));
        assert!(is_self_closing("input"));
        assert!(!is_self_closing("div"));
        assert!(!is_self_closing("p"));
    }

    #[test]
    fn test_simple_element_text_content() {
        let mut host = FakeHost::new();
        let el = make_element(
            vibao_ast::Tag::Text,
            vec![("noi_dung".to_string(), Expr::literal_str("Xin chào", p()))],
        );
        let html = gen_simple_element(&el, "vb-text-1", &mut host);
        assert!(html.starts_with("<p "));
        assert!(html.contains(">Xin chào</p>"));
    }

    #[test]
    fn test_simple_element_self_closing_image() {
        let mut host = FakeHost::new();
        let el = make_element(
            vibao_ast::Tag::Image,
            vec![("mo_ta_anh".to_string(), Expr::literal_str("logo", p()))],
        );
        let html = gen_simple_element(&el, "vb-image-1", &mut host);
        assert!(html.starts_with("<img "));
        assert!(html.ends_with("/>"));
    }

    #[test]
    fn test_simple_element_dynamic_content_uses_span_binding() {
        let mut host = FakeHost::new();
        let el = make_element(
            vibao_ast::Tag::Text,
            vec![(
                "noi_dung".to_string(),
                Expr::Variable("ten".to_string(), p()),
            )],
        );
        let html = gen_simple_element(&el, "vb-text-1", &mut host);
        assert!(html.contains("data-vb-text="));
    }

    #[test]
    fn test_layout_element_is_div_with_css_registered() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Flex, vec![]);
        let html = gen_layout_element(&el, "vb-flex-1", &mut host);
        assert!(html.starts_with("<div id=\"vb-flex-1\""));
        assert_eq!(host.css.len(), 1);
        assert!(host.css[0].contains("display: flex;"));
    }

    #[test]
    fn test_layout_element_layer_gets_relative_position() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Lop, vec![]);
        let html = gen_layout_element(&el, "vb-layer-1", &mut host);
        assert!(html.contains("position:relative;"));
    }

    #[test]
    fn test_layout_element_warns_on_unknown_prop() {
        let props: vibao_ast::PropsMap = vec![(
            "magin".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("8".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Khoi, props);
        let mut host = FakeHost::new();
        gen_layout_element(&el, "vb-box-1", &mut host);
        assert_eq!(host.warnings.len(), 1);
        assert!(host.warnings[0].contains("magin"));
        assert!(host.warnings[0].contains("khoi"));
    }

    #[test]
    fn test_layout_element_no_warning_for_data_prefixed_prop() {
        let props: vibao_ast::PropsMap = vec![(
            "data_testid".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("hero".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Khoi, props);
        let mut host = FakeHost::new();
        gen_layout_element(&el, "vb-box-1", &mut host);
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn test_layout_element_no_warning_for_aria_prefixed_prop() {
        let props: vibao_ast::PropsMap = vec![(
            "aria_hidden".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("true".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Flex, props);
        let mut host = FakeHost::new();
        gen_layout_element(&el, "vb-flex-1", &mut host);
        assert!(host.warnings.is_empty());
    }

    fn make_element_with_responsive(
        tag: vibao_ast::Tag,
        responsive: Vec<ResponsiveNode>,
    ) -> Element {
        Element {
            tag,
            content: None,
            identity: Default::default(),
            blocks: vec![],
            unknown_props: vec![],
            children: vec![],
            events: vec![],
            responsive,
            animation: AnimationProps::default(),
            pos: p(),
        }
    }

    #[test]
    fn test_responsive_warns_on_unknown_prop_simple_element() {
        let el = make_element_with_responsive(
            vibao_ast::Tag::Text,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![("ronnng".to_string(), Expr::literal_num(100.0, p()))],
                pos: p(),
            }],
        );
        let mut host = FakeHost::new();
        gen_simple_element(&el, "vb-text-1", &mut host);
        assert_eq!(host.warnings.len(), 1);
        assert!(host.warnings[0].contains("ronnng"));
    }

    #[test]
    fn test_responsive_warns_on_unknown_prop_layout_element() {
        let el = make_element_with_responsive(
            vibao_ast::Tag::Khoi,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![("ronnng".to_string(), Expr::literal_num(100.0, p()))],
                pos: p(),
            }],
        );
        let mut host = FakeHost::new();
        gen_layout_element(&el, "vb-box-1", &mut host);
        assert_eq!(host.warnings.len(), 1);
        assert!(host.warnings[0].contains("ronnng"));
    }

    #[test]
    fn test_responsive_known_vocabulary_does_not_warn() {
        let overrides: vibao_ast::PropsMap = vec![
            ("cot".to_string(), Expr::literal_num(2.0, p())),
            (
                "huong".to_string(),
                Expr::Literal(vibao_ast::LiteralValue::Str("row".to_string()), p()),
            ),
            ("co".to_string(), Expr::literal_num(14.0, p())),
            ("rong".to_string(), Expr::literal_num(100.0, p())),
            ("cao".to_string(), Expr::literal_num(100.0, p())),
            ("dem".to_string(), Expr::literal_num(8.0, p())),
            (
                "an".to_string(),
                Expr::Literal(vibao_ast::LiteralValue::Str("true".to_string()), p()),
            ),
        ];
        let el = make_element_with_responsive(
            vibao_ast::Tag::Text,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides,
                pos: p(),
            }],
        );
        let mut host = FakeHost::new();
        gen_simple_element(&el, "vb-text-1", &mut host);
        assert!(
            host.warnings.is_empty(),
            "7 prop hợp lệ không được cảnh báo: {:?}",
            host.warnings
        );
    }

    #[test]
    fn test_responsive_no_warning_for_data_prefixed_prop() {
        let el = make_element_with_responsive(
            vibao_ast::Tag::Text,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![(
                    "data_testid".to_string(),
                    Expr::Literal(vibao_ast::LiteralValue::Str("hero".to_string()), p()),
                )],
                pos: p(),
            }],
        );
        let mut host = FakeHost::new();
        gen_simple_element(&el, "vb-text-1", &mut host);
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn test_responsive_wrong_tag_for_known_vocab_not_flagged_documented_limitation() {
        let el = make_element_with_responsive(
            vibao_ast::Tag::Text,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![("cot".to_string(), Expr::literal_num(2.0, p()))],
                pos: p(),
            }],
        );
        let mut host = FakeHost::new();
        gen_simple_element(&el, "vb-text-1", &mut host);
        assert!(
            host.warnings.is_empty(),
            "giới hạn đã biết: mức vocabulary không bắt được sai-tag"
        );
    }

    #[test]
    fn test_responsive_warning_message_distinguishes_valid_propkey_from_typo() {
        let el_valid_propkey = make_element_with_responsive(
            vibao_ast::Tag::Khoi,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![("bong".to_string(), Expr::literal_str("true", p()))],
                pos: p(),
            }],
        );
        let mut host1 = FakeHost::new();
        gen_layout_element(&el_valid_propkey, "vb-box-1", &mut host1);
        assert_eq!(host1.warnings.len(), 1);
        assert!(
            host1.warnings[0].contains("hợp lệ"),
            "'bong' là PropKey hợp lệ, message phải nói rõ KHÔNG phải gõ sai: {}",
            host1.warnings[0]
        );
        assert!(
            !host1.warnings[0].contains("có thể do gõ sai"),
            "'bong' không nên bị nghi ngờ là gõ sai: {}",
            host1.warnings[0]
        );

        let el_typo = make_element_with_responsive(
            vibao_ast::Tag::Khoi,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![("bonng".to_string(), Expr::literal_str("true", p()))],
                pos: p(),
            }],
        );
        let mut host2 = FakeHost::new();
        gen_layout_element(&el_typo, "vb-box-2", &mut host2);
        assert_eq!(host2.warnings.len(), 1);
        assert!(
            host2.warnings[0].contains("có thể do gõ sai"),
            "'bonng' không phải PropKey nào, message phải nghi ngờ gõ sai: {}",
            host2.warnings[0]
        );
    }

    #[test]
    fn test_responsive_unknown_keys_deduped_across_breakpoints() {
        let el = make_element_with_responsive(
            vibao_ast::Tag::Text,
            vec![
                ResponsiveNode {
                    breakpoint: Breakpoint::DiDong,
                    overrides: vec![("ronnng".to_string(), Expr::literal_num(100.0, p()))],
                    pos: p(),
                },
                ResponsiveNode {
                    breakpoint: Breakpoint::MayTinhBang,
                    overrides: vec![("ronnng".to_string(), Expr::literal_num(200.0, p()))],
                    pos: p(),
                },
            ],
        );
        let mut host = FakeHost::new();
        gen_simple_element(&el, "vb-text-1", &mut host);
        assert_eq!(
            host.warnings.len(),
            1,
            "phải dedupe, chỉ 1 cảnh báo dù 'ronnng' xuất hiện ở 2 breakpoint"
        );
    }

    #[test]
    fn test_layout_element_dynamic_width_generates_binding_not_dead_static_value() {
        let mut host = FakeHost::new();
        let el = make_element(
            vibao_ast::Tag::Khoi,
            vec![(
                "rong".to_string(),
                Expr::Variable("do_rong".to_string(), p()),
            )],
        );
        let html = gen_layout_element(&el, "vb-box-1", &mut host);
        assert!(
            html.contains("data-vb-style-width=\""),
            "phải có binding động cho width: {}",
            html
        );
        assert!(
            !html.contains("__dynamic__"),
            "sentinel không được lọt vào output cuối: {}",
            html
        );
    }

    #[test]
    fn test_layout_element_hover_animation_no_longer_double_applied() {
        let mut host = FakeHost::new();
        let mut el = make_element(vibao_ast::Tag::Khoi, vec![]);
        el.animation.hieu_ung_hover = Some("phong_to".to_string());
        let html = gen_layout_element(&el, "vb-box-1", &mut host);
        assert!(
            html.contains("data-vb-anim-hover=\"phong_to:300\""),
            "phải có attr hover: {}",
            html
        );
        assert!(
            host.js.is_empty(),
            "KHÔNG được add_js cho hover animation (cơ chế cũ đã bỏ): {:?}",
            host.js
        );
    }

    #[test]
    fn test_layout_element_scroll_animation_no_longer_double_applied() {
        let mut host = FakeHost::new();
        let mut el = make_element(vibao_ast::Tag::Khoi, vec![]);
        el.animation.hieu_ung_cuon = Some("truot_len".to_string());
        let html = gen_layout_element(&el, "vb-box-1", &mut host);
        assert!(
            html.contains("data-vb-anim-scroll=\"truot_len:600:0\""),
            "phải có attr scroll: {}",
            html
        );
        assert!(
            host.js.is_empty(),
            "KHÔNG được add_js cho scroll animation (cơ chế cũ đã bỏ): {:?}",
            host.js
        );
    }

    #[test]
    fn test_complex_component_placeholder() {
        let el = make_element(vibao_ast::Tag::Modal, vec![]);
        let html = gen_complex_component(&el, "vb-modal-1");
        assert_eq!(
            html,
            "<div id=\"vb-modal-1\" data-vb-component=\"modal\"><!-- modal --></div>"
        );
    }

    #[test]
    fn test_gen_element_dispatches_to_complex_for_builtin() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Tabs, vec![]);
        let html = gen_element(&el, &mut host);
        assert!(html.contains("data-vb-component=\"tabs\""));
    }

    #[test]
    fn test_gen_element_dispatches_to_layout_for_flex() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Flex, vec![]);
        let html = gen_element(&el, &mut host);
        assert!(html.starts_with("<div id=\"vb-flex-1\""));
    }

    #[test]
    fn test_gen_anim_attrs_hover_only() {
        let anim = AnimationProps {
            hieu_ung: None,
            thoi_gian: Some(400),
            tre: None,
            lap: None,
            hieu_ung_hover: Some("phong_to".to_string()),
            hieu_ung_cuon: None,
        };
        let out = gen_anim_attrs(&anim);
        assert_eq!(out, "data-vb-anim-hover=\"phong_to:400\"");
    }

    #[test]
    fn test_gen_anim_attrs_hover_default_duration() {
        let anim = AnimationProps {
            hieu_ung_hover: Some("phong_to".to_string()),
            ..AnimationProps::default()
        };
        let out = gen_anim_attrs(&anim);
        assert_eq!(out, "data-vb-anim-hover=\"phong_to:300\"");
    }

    #[test]
    fn test_gen_anim_attrs_scroll_with_delay() {
        let anim = AnimationProps {
            hieu_ung_cuon: Some("truot_len".to_string()),
            thoi_gian: Some(500),
            tre: Some(150),
            ..AnimationProps::default()
        };
        let out = gen_anim_attrs(&anim);
        assert_eq!(out, "data-vb-anim-scroll=\"truot_len:500:150\"");
    }

    #[test]
    fn test_gen_anim_attrs_combines_load_and_hover() {
        let anim = AnimationProps {
            hieu_ung: Some("fade_in".to_string()),
            thoi_gian: Some(200),
            hieu_ung_hover: Some("phong_to".to_string()),
            ..AnimationProps::default()
        };
        let out = gen_anim_attrs(&anim);
        assert!(out.contains("data-vb-anim=\"fade_in\""));
        assert!(out.contains("data-vb-anim-hover=\"phong_to:200\""));
    }

    #[test]
    fn test_gen_anim_attrs_empty_without_hieu_ung() {
        let anim = AnimationProps::default();
        assert_eq!(gen_anim_attrs(&anim), "");
    }

    #[test]
    fn test_gen_anim_attrs_full() {
        let anim = AnimationProps {
            hieu_ung: Some("fade_in".to_string()),
            thoi_gian: Some(500),
            tre: Some(100),
            lap: Some(LapValue::MaiMai),
            hieu_ung_hover: None,
            hieu_ung_cuon: None,
        };
        let out = gen_anim_attrs(&anim);
        assert!(out.contains("data-vb-anim=\"fade_in\""));
        assert!(out.contains("data-vb-anim-duration=\"500\""));
        assert!(out.contains("data-vb-anim-delay=\"100\""));
        assert!(out.contains("data-vb-anim-repeat=\"infinite\""));
    }

    #[test]
    fn test_make_class_name() {
        assert_eq!(make_class_name("khoi", 3), "vb-box-3");
    }

    #[test]
    fn test_gen_loop_hoists_template_instead_of_inlining() {
        let mut host = FakeHost::new();
        let node = LoopNode {
            kind: LoopKind::Range {
                from: 1,
                to: 3,
                var_name: "i".to_string(),
            },
            body: vec![Child::Element(make_element(vibao_ast::Tag::Text, vec![]))],
            pos: p(),
        };
        let html = crate::codegen::control::gen_loop(&node, &mut host);

        assert!(
            !html.contains("<template"),
            "template không được nằm trong HTML trả về trực tiếp: {}",
            html
        );
        assert!(
            html.contains("data-vb-loop="),
            "phải có container binding: {}",
            html
        );
        assert_eq!(
            host.hoisted_templates.len(),
            1,
            "phải có đúng 1 template được hoisted"
        );
        assert!(host.hoisted_templates[0].contains("<template"));
    }

    #[test]
    fn test_nested_gen_loop_produces_two_separate_hoisted_templates_with_distinct_ids() {
        let mut host = FakeHost::new();
        let inner_loop = Child::Loop(Box::new(LoopNode {
            kind: LoopKind::Range {
                from: 1,
                to: 2,
                var_name: "i".to_string(),
            },
            body: vec![Child::Element(make_element(vibao_ast::Tag::Text, vec![]))],
            pos: p(),
        }));
        let outer_node = LoopNode {
            kind: LoopKind::Range {
                from: 1,
                to: 3,
                var_name: "i".to_string(),
            },
            body: vec![inner_loop],
            pos: p(),
        };
        let outer_html = crate::codegen::control::gen_loop(&outer_node, &mut host);

        assert!(
            !outer_html.contains("<template"),
            "outer_html: {}",
            outer_html
        );

        assert_eq!(
            host.hoisted_templates.len(),
            2,
            "phải có đúng 2 template tách biệt, không lồng nhau"
        );

        let extract_id = |html: &str| -> String {
            let start = html.find("id=\"").unwrap() + 4;
            let end = html[start..].find('"').unwrap() + start;
            html[start..end].to_string()
        };
        let id1 = extract_id(&host.hoisted_templates[0]);
        let id2 = extract_id(&host.hoisted_templates[1]);
        assert_ne!(id1, id2, "2 template hoisted không được trùng ID");

        for t in &host.hoisted_templates {
            assert_eq!(
                t.matches("<template").count(),
                1,
                "mỗi template hoisted phải chỉ chứa đúng 1 <template>, không lồng cái khác: {}",
                t
            );
        }
    }
}
