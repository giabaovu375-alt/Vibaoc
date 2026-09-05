// ============================================================
// VIBAO COMPILER (Rust) — lexer/vocabulary.rs
//
// The SINGLE SOURCE OF TRUTH for the WITH-DIACRITICS spelling of every
// keyword, color name, and component name in ViBao — this is the
// CANONICAL FORM that docs/examples should use when showing dev how to
// write ViBao code.
//
// IMPORTANT — READ BEFORE CHANGING ANYTHING HERE:
// This file does NOT change how the lexer behaves. `keyword_map()` /
// `color_map()` / `component_set()` in tables.rs still use the
// DIACRITICS-FREE form (ASCII, snake_case) — that's the "internal
// normalized form" that every identifier (whether typed with
// diacritics, without them, or via the multi-word quote syntax) is
// reduced to by `normalize_vietnamese()` (see vietnamese.rs) BEFORE any
// table lookup. These two concerns are deliberately kept separate:
//   - tables.rs: the REAL lookup tables, used directly by the lexer,
//     already covered by plenty of tests — shouldn't be touched unless
//     truly necessary.
//   - vocabulary.rs (THIS FILE): DOCUMENTATION + VERIFICATION — clearly
//     lists the "official" with-diacritics form of each word, with one
//     test (`test_every_vocabulary_entry_normalizes_correctly`)
//     confirming EVERY entry here actually strips down to match
//     tables.rs. If someone adds a new keyword to tables.rs and forgets
//     to add its with-diacritics form here, NOTHING will fail (this is
//     only documentation) — but if someone adds a WRONG entry here (a
//     with-diacritics form that doesn't strip to the registered ASCII
//     form), the test fails immediately.
//
// Why tables.rs isn't changed to use diacritics directly: much higher
// risk (would require changing every matching string throughout the
// entire lexer/parser, easy to miss one), and unnecessary from a
// technical standpoint — normalize_vietnamese() already correctly
// solves the "written with diacritics" problem without needing to
// change a single line of logic anywhere else in the compiler.
// ============================================================

/// (the official WITH-DIACRITICS form, the ASCII form registered in
/// tables.rs). This is a REFERENCE/DOCUMENTATION table — never read
/// directly by the lexer.
#[cfg(test)]
pub const KEYWORDS: &[(&str, &str)] = &[
    ("trang", "trang"),
    ("ứng dụng", "ung_dung"),
    ("nhập", "nhap"),
    ("từ", "tu"),
    ("nếu", "neu"),
    ("không thì", "khong_thi"),
    ("nếu nhiều", "neu_nhieu"),
    ("trường hợp", "truong_hop"),
    ("mặc định", "mac_dinh"),
    ("vòng lặp", "vong_lap"),
    ("on_tải", "on_tai"),
    ("on_hủy", "on_huy"),
    // The events below borrow their English names as-is (on_click,
    // on_hover...) — there's no with-diacritics form since "click"/
    // "hover" aren't Vietnamese words to begin with.
];

#[cfg(test)]
pub const COMPONENTS: &[(&str, &str)] = &[
    ("nhãn", "nhan"),
    ("liên kết", "lien_ket"),
    ("đính đầu", "dinh_dau"),
    ("đính màn hình", "dinh_man_hinh"),
    ("nhóm input", "nhom_input"),
    ("chọn một", "chon_mot"),
    ("hộp kiểm", "hop_kiem"),
    ("lựa chọn", "lua_chon"),
    ("xuống trang", "xuong_trang"),
    ("vòng quay", "vong_quay"),
    ("thanh tiến trình", "thanh_tien_trinh"),
    ("bảng", "bang"),
    ("biểu đồ", "bieu_do"),
    ("bản đồ", "ban_do"),
    ("trình soạn thảo", "trinh_soan_thao"),
    ("thông báo", "thong_bao"),
    ("cảnh báo", "canh_bao"),
    ("điều hướng", "dieu_huong"),
    ("mở tab mới", "mo_tab_moi"),
    ("mở modal", "mo_modal"),
    ("đóng modal", "dong_modal"),
    ("cuộn đến", "cuon_den"),
    ("cuộn lên đầu", "cuon_len_dau"),
    ("lưu dữ liệu", "luu_du_lieu"),
    ("tải dữ liệu", "tai_du_lieu"),
    // "dang xuat" (log out) has been REMOVED from this table (as part
    // of removing dang_xuat from component_set() - see
    // AUDIT.md/tables.rs) - "dang_xuat" is NO LONGER a valid action (no
    // runtime handler exists, this decision is settled), so it is no
    // longer recognized by component_set(). If an entry were kept here,
    // test_every_component_ascii_form_exists_in_component_set (at the
    // bottom of this file) would fail because "dang_xuat" is no longer
    // in the real component_set() - this file MUST always match
    // tables.rs exactly; it is not a historical archive.
    ("sao chép", "sao_chep"),
    ("giá tiền", "gia_tien"),
    ("ngày", "ngay"),
    ("rút gọn", "rut_gon"),
    ("hoa chữ", "hoa_chu"),
    ("phần trăm", "phan_tram"),
    ("làm tròn", "lam_tron"),
    ("gọi api", "goi_api"),
    ("thêm vào mảng", "them_vao_mang"),
    ("xoá theo id", "xoa_theo_id"),
    ("cập nhật theo id", "cap_nhat_theo_id"),
    // NEWLY Vietnamese-ized tags/components (per the settled vocabulary
    // table - these previously kept their English names, now changed
    // since they aren't standard international terminology, and the new
    // names reflect their FUNCTION rather than their implementation):
    ("khối", "khoi"),
    ("cuộn", "cuon"),
    ("lớp", "lop"),
    ("khoảng cách", "khoang_cach"),
    ("đường kẻ", "duong_ke"),
    ("gấp mở", "gap_mo"),
    ("băng chuyền", "bang_chuyen"),
    ("thanh điều hướng", "thanh_dieu_huong"),
    ("căn giữa", "can_giua"),
    // "text", "h1", "h2", "h3", "p", "image", "video", "icon", "button",
    // "input", "flex", "grid", "stack", "form", "modal", "tabs" -
    // deliberately borrow their English names as-is (a design decision,
    // not an oversight): these are common international terms in web
    // development (anyone learning another language/framework will run
    // into them eventually), keeping them unchanged means learners don't
    // have to relearn these terms when moving to another language. Only
    // tags that AREN'T standard/specialized technical terminology were
    // Vietnamese-ized (box/container/layer/spacer/divider/accordion/
    // carousel/nav above).
];

#[cfg(test)]
pub const COLORS: &[(&str, &str)] = &[
    ("đỏ", "do"),
    ("xanh lá", "xanh_la"),
    ("vàng", "vang"),
    ("hồng", "hong"),
    ("tím", "tim"),
    ("xám", "xam"),
    ("xám nhạt", "xam_nhat"),
    ("xám đậm", "xam_dam"),
    ("lục", "luc"),
    ("nâu", "nau"),
    ("đen", "den"),
    // "trang" (white), "xanh", "cam" are already naturally
    // diacritics-free or have only one form, no separate with-diacritics
    // entry is needed.
];

/// A prop (an attribute passed into an element/layout tag, e.g.
/// `mau_nen: xanh`) - kept SEPARATE from COMPONENTS (which is for TAGS,
/// e.g. `text`/`khoi`) because props are handled by the lexer/parser at
/// a DIFFERENT LAYER (not through `component_set()`). Semantic
/// resolution happens through `PropKey`/`PropSpec`; this table only
/// documents the with-diacritics Vietnamese form. Merging these two
/// groups into one array (as happened once before) caused
/// `test_every_component_ascii_form_exists_in_component_set` to FAIL
/// INCORRECTLY (it went looking for a PROP name in the TAG list, which
/// obviously wasn't there) - lesson: when adding a new entry, always
/// confirm which GROUP/MEANING it belongs to before adding it to any
/// array.
#[cfg(test)]
pub const PROPS: &[(&str, &str)] = &[
    ("màu nền", "mau_nen"),
    ("rộng", "rong"),
    // "cao" is already naturally diacritics-free, no separate entry needed.
    ("phông chữ", "phong_chu"),
    ("khớp", "khop"),
    ("nguồn", "nguon"),
    ("mô tả ảnh", "mo_ta_anh"),
    ("tải chậm", "tai_cham"),
    // "overflow" is deliberately named "cuon_tran" (NOT "tran") -
    // "tran" was already taken by "tran_x"/"tran_y" (transform-translate),
    // and reusing it would confuse two completely different concepts
    // (content overflowing a box vs. translating its position).
    ("cuộn tràn", "cuon_tran"),
    // `lop` has two semantic domains: Tag::Lop = layer and
    // PropKey::ClassBinding = CSS class. The lexer normalizes both to
    // the same surface name; the parser/codegen distinguish them by
    // whether they appear in tag or prop position.
    ("lớp", "lop"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::vietnamese::normalize_vietnamese;

    /// The MOST IMPORTANT test in this file: confirms EVERY entry in
    /// the 3 tables above actually strips down to EXACTLY its registered
    /// ASCII form. If this test passes, it means the entire current
    /// keyword/component/color list can ALL already be written with
    /// Vietnamese diacritics right now, with nothing more to change in
    /// the lexer core - the only thing missing was documenting it (which
    /// is exactly this file's purpose).
    #[test]
    fn test_every_vocabulary_entry_normalizes_correctly() {
        let mut failures = Vec::new();
        for (diacritic, expected_ascii) in KEYWORDS.iter().chain(COMPONENTS).chain(COLORS).chain(PROPS) {
            let got = normalize_vietnamese(diacritic);
            if got != *expected_ascii {
                failures.push(format!(
                    "'{}' normalizes to '{}', expected '{}'",
                    diacritic, got, expected_ascii
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "{} entries in vocabulary.rs do NOT match the form registered in tables.rs:\n{}",
            failures.len(),
            failures.join("\n"),
        );
    }

    /// Confirms every (diacritic, ascii) pair in vocabulary.rs ACTUALLY
    /// exists in the corresponding lookup table in tables.rs - if
    /// vocabulary.rs accidentally has a nonexistent ASCII form (a typo),
    /// this test catches it, unlike the test above (which only checks
    /// that normalization is correct, not that the ASCII form is
    /// ACTUALLY registered in tables.rs).
    #[test]
    fn test_every_keyword_ascii_form_exists_in_keyword_map() {
        let km = crate::lexer::tables::keyword_map();
        for (diacritic, expected_ascii) in KEYWORDS {
            assert!(
                km.contains_key(expected_ascii),
                "'{}' (from '{}') does not exist in the real keyword_map()",
                expected_ascii, diacritic,
            );
        }
    }

    #[test]
    fn test_every_component_ascii_form_exists_in_component_set() {
        let cs = crate::lexer::tables::component_set();
        for (diacritic, expected_ascii) in COMPONENTS {
            assert!(
                cs.contains(expected_ascii),
                "'{}' (from '{}') does not exist in the real component_set()",
                expected_ascii, diacritic,
            );
        }
    }

    #[test]
    fn test_every_color_ascii_form_exists_in_color_map() {
        let cm = crate::lexer::tables::color_map();
        for (diacritic, expected_ascii) in COLORS {
            assert!(
                cm.contains_key(expected_ascii),
                "'{}' (from '{}') does not exist in the real color_map()",
                expected_ascii, diacritic,
            );
        }
    }

    /// A semantic regression test for PROPS - doesn't rely on a second
    /// surface table. Every documented ASCII name must resolve to a
    /// PropKey and must be marked valid for Simple Element by
    /// PropSpec.
    #[test]
    fn test_every_prop_ascii_form_resolves_as_simple_prop() {
        for (diacritic, expected_ascii) in PROPS {
            let key = crate::locale::resolve_prop_key(expected_ascii)
                .unwrap_or_else(|| panic!("'{}' (from '{}') does not resolve to a PropKey", expected_ascii, diacritic));
            assert!(
                vibao_ast::prop_spec(key).applies_to_simple,
                "'{}' (from '{}') resolves but is not a Simple prop",
                expected_ascii, diacritic,
            );
        }
    }
}
