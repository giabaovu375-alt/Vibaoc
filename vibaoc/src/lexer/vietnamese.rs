pub fn normalize_vietnamese(input: &str) -> String {
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
        if c.is_whitespace() {
            result.push('_');
        } else {
            result.extend(c.to_lowercase());
        }
    }

    result
}

fn is_combining_mark(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c)
}

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

trait NfdNormalizeVi {
    fn nfd_normalize(&self) -> String;
}

impl NfdNormalizeVi for String {
    fn nfd_normalize(&self) -> String {
        self.chars().map(decompose_vietnamese_char).collect::<Vec<String>>().join("")
    }
}

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
        other => return other.to_string(),
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
