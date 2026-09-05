// ============================================================
// VIBAO COMPILER (Rust) — lexer/vietnamese.rs
//
// Normalizes an identifier written WITH Vietnamese diacritics into the
// exact diacritics-free snake_case form already defined by
// keyword_map()/color_map()/component_set() — with NO changes needed to
// any of those lookup tables.
//
// EXAMPLES:
//   "mau"        -> "mau"          (already unaccented)
//   "mau chu"    -> "mau_chu"
//   "xanh duong" -> "xanh_duong"
//   "Dam"        -> "dam"   (uppercase is also normalized to lowercase,
//                            since every current ViBao keyword is
//                            lowercase)
//
// ONLY applies to IDENTIFIERS (called from read_identifier() in
// scan.rs), NOT to string literals (scan_string is a completely
// separate path that never calls this function) — so "Xin chao" inside
// text("Xin chao") keeps its diacritics when displayed on the web,
// consistent with ViBao's original purpose as a Vietnamese-language
// programming language.
//
// WHY NOT CHANGE keyword_map()/color_map() TO USE DIACRITICS DIRECTLY:
// much higher risk (would require editing every matching string across
// many files, easy to miss one, and would be a BREAKING CHANGE for the
// diacritics-free strings that still need to keep working). Normalizing
// the input ONCE, here, is cheaper, safer, and doesn't touch any
// already well-tested logic.
// ============================================================

/// Strips diacritics from a Vietnamese string, while also lowercasing
/// it and turning whitespace into `_` — turns "Mau Chu" or "mau chu"
/// into "mau_chu" either way.
///
/// The character `d(with stroke)`/uppercase equivalent is handled
/// MANUALLY before Unicode decomposition (NFD), because it is its own
/// separate letter in the Unicode table (U+0111/U+0110), NOT "d" plus a
/// diacritic — skipping this step would leave it unrecognized as
/// needing normalization, and it would be left as-is, breaking table
/// lookups (keyword_map() has no key containing that character).
pub fn normalize_vietnamese(input: &str) -> String {
    let step1: String = input
        .chars()
        .map(|c| match c {
            'đ' => 'd',
            'Đ' => 'D',
            other => other,
        })
        .collect();

    // Unicode decomposition: whether the input has a precomposed
    // circumflex-e (1 code point) or "e" + a separate circumflex mark (2
    // code points), NFD returns "e" + its own combining character
    // either way — the filtering step below strips every combining
    // character, keeping only the base unaccented letter.
    let decomposed: String = step1.nfd_normalize();

    let mut result = String::with_capacity(decomposed.len());
    for c in decomposed.chars() {
        if is_combining_mark(c) {
            continue;
        }
        if c.is_whitespace() {
            result.push('_');
        } else {
            result.extend(c.to_lowercase());
        }
    }

    result
}

/// Checks whether a character is a combining diacritical mark (the
/// Vietnamese tone marks and vowel modifiers) — based on the Unicode
/// range U+0300-U+036F (Combining Diacritical Marks), which covers
/// every Vietnamese diacritic once NFD-decomposed.
fn is_combining_mark(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c)
}

/// Same as `normalize_vietnamese`, but PRESERVES the original casing —
/// only strips Vietnamese diacritics, does NOT lowercase, does NOT turn
/// spaces into `_`. Used specifically for the ACTUAL VALUE stored in a
/// token when the lexer recognizes this as a dev-chosen identifier
/// (component/variable name), unlike `normalize_vietnamese` (used to
/// CHECK the keyword/color/component tables, where casing doesn't
/// matter and everything is lowercased for consistent matching).
///
/// EXAMPLE: "TheBao" -> "TheBao" (unchanged, no diacritics to begin
/// with), "Do" (with diacritic) -> "Do" (diacritic stripped, uppercase D
/// kept).
pub fn strip_diacritics_keep_case(input: &str) -> String {
    let step1: String = input
        .chars()
        .map(|c| match c {
            'đ' => 'd',
            'Đ' => 'D',
            other => other,
        })
        .collect();

    let decomposed: String = step1.nfd_normalize();

    let mut result = String::with_capacity(decomposed.len());
    for c in decomposed.chars() {
        if is_combining_mark(c) {
            continue;
        }
        result.push(c);
    }

    result
}

/// A small internal trait for doing NFD decomposition WITHOUT adding an
/// external dependency (the `unicode-normalization` crate) — the
/// project deliberately keeps dependencies minimal (see
/// CONTRIBUTING.md). The mapping table below only covers the accented
/// vowels/consonants that ACTUALLY occur in Vietnamese (not attempting
/// a fully general Unicode NFD implementation — unnecessary for the
/// scope of a single programming language).
trait NfdNormalizeVi {
    fn nfd_normalize(&self) -> String;
}

impl NfdNormalizeVi for String {
    fn nfd_normalize(&self) -> String {
        self.chars().map(decompose_vietnamese_char).collect::<Vec<String>>().join("")
    }
}

/// Maps a precomposed accented Vietnamese character (the most common
/// form produced by real-world Vietnamese typing methods like
/// Telex/VNI) to the string "base letter + a simulated combining mark"
/// — using U+0301 (COMBINING ACUTE ACCENT) as a GENERIC marker for
/// every tone, since the is_combining_mark() filter above treats the
/// ENTIRE U+0300-U+036F range as something to strip; there's no need to
/// distinguish which specific tone it was — the end goal is only
/// stripping diacritics, not phonetic analysis.
fn decompose_vietnamese_char(c: char) -> String {
    const MARK: char = '\u{0301}';
    let base = match c {
        'à' | 'á' | 'ả' | 'ã' | 'ạ' => 'a',
        'ằ' | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' => 'a',
        'ầ' | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' => 'a',
        'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' => 'e',
        'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' => 'e',
        'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' => 'i',
        'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' => 'o',
        'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' => 'o',
        'ờ' | 'ớ' | 'ở' | 'ỡ' | 'ợ' => 'o',
        'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' => 'u',
        'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' => 'u',
        'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' => 'y',
        'ă' => 'a',
        'â' => 'a',
        'ê' => 'e',
        'ô' => 'o',
        'ơ' => 'o',
        'ư' => 'u',
        // Uppercase — same logic, just with an uppercase base letter.
        'À' | 'Á' | 'Ả' | 'Ã' | 'Ạ' => 'A',
        'Ằ' | 'Ắ' | 'Ẳ' | 'Ẵ' | 'Ặ' => 'A',
        'Ầ' | 'Ấ' | 'Ẩ' | 'Ẫ' | 'Ậ' => 'A',
        'È' | 'É' | 'Ẻ' | 'Ẽ' | 'Ẹ' => 'E',
        'Ề' | 'Ế' | 'Ể' | 'Ễ' | 'Ệ' => 'E',
        'Ì' | 'Í' | 'Ỉ' | 'Ĩ' | 'Ị' => 'I',
        'Ò' | 'Ó' | 'Ỏ' | 'Õ' | 'Ọ' => 'O',
        'Ồ' | 'Ố' | 'Ổ' | 'Ỗ' | 'Ộ' => 'O',
        'Ờ' | 'Ớ' | 'Ở' | 'Ỡ' | 'Ợ' => 'O',
        'Ù' | 'Ú' | 'Ủ' | 'Ũ' | 'Ụ' => 'U',
        'Ừ' | 'Ứ' | 'Ử' | 'Ữ' | 'Ự' => 'U',
        'Ỳ' | 'Ý' | 'Ỷ' | 'Ỹ' | 'Ỵ' => 'Y',
        'Ă' => 'A',
        'Â' => 'A',
        'Ê' => 'E',
        'Ô' => 'O',
        'Ơ' => 'O',
        'Ư' => 'U',
        other => return other.to_string(), // not an accented character, pass through
    };
    format!("{}{}", base, MARK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_word_no_diacritics_unchanged() {
        assert_eq!(normalize_vietnamese("mau"), "mau");
    }

    #[test]
    fn test_single_diacritic_word() {
        assert_eq!(normalize_vietnamese("màu"), "mau");
    }

    #[test]
    fn test_dd_special_case() {
        // "d" (with stroke) is NOT "d" + a diacritic - it must be
        // handled separately; this is the most important test since
        // this mistake is easy to miss if you only think in terms of
        // ordinary NFD decomposition.
        assert_eq!(normalize_vietnamese("đậm"), "dam");
        assert_eq!(normalize_vietnamese("Đậm"), "dam");
    }

    #[test]
    fn test_multi_word_with_space_becomes_underscore() {
        assert_eq!(normalize_vietnamese("màu chữ"), "mau_chu");
        assert_eq!(normalize_vietnamese("xanh dương"), "xanh_duong");
    }

    #[test]
    fn test_uppercase_normalized_to_lowercase() {
        assert_eq!(normalize_vietnamese("Màu Chữ"), "mau_chu");
        assert_eq!(normalize_vietnamese("XANH"), "xanh");
    }

    #[test]
    fn test_circumflex_and_horn_vowels() {
        assert_eq!(normalize_vietnamese("dương"), "duong");
        assert_eq!(normalize_vietnamese("viền"), "vien");
        assert_eq!(normalize_vietnamese("gạch_chân"), "gach_chan");
    }

    #[test]
    fn test_all_five_tone_marks_on_same_vowel() {
        // Confirms all 5 tone marks (level/falling/rising/dipping-rising/
        // creaky-rising/heavy) map to the same base vowel - no tone is
        // missed.
        for (input, expected) in [
            ("ba", "ba"), ("bà", "ba"), ("bá", "ba"), ("bả", "ba"), ("bã", "ba"), ("bạ", "ba"),
        ] {
            assert_eq!(normalize_vietnamese(input), expected, "input: {}", input);
        }
    }

    #[test]
    fn test_already_ascii_untouched() {
        assert_eq!(normalize_vietnamese("nut_bam123"), "nut_bam123");
    }
}
