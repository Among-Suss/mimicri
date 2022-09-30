use unicode_segmentation::UnicodeSegmentation;

static SENSITIVE_CHARACTERS: [&str; 7] = ["\\", "*", "_", "~", "`", "|", ">"];

/// Escapes all sensitize Discord characters
/* @see: https://github.com/discord-net/Discord.Net/blob/265da99619a775d23b24326648fe4220bc6beeae/src/Discord.Net.Core/Format.cs#L36 */
pub fn escape_string(text: &String) -> String {
    let mut sanitized_text = text.clone();

    for i in 0..SENSITIVE_CHARACTERS.len() {
        sanitized_text = sanitized_text.replace(
            SENSITIVE_CHARACTERS[i],
            format!("\\{}", SENSITIVE_CHARACTERS[i]).as_str(),
        );
    }

    sanitized_text
}

pub fn limit_string_length(text: &String, width: usize) -> String {
    let graphemes = text.graphemes(true).collect::<Vec<&str>>();
    let length = graphemes.len();

    if width == 1 {
        return graphemes[0].to_string();
    }

    if length <= width {
        return graphemes.join("").to_string();
    }

    graphemes[0..width - 1].join("").to_string() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_string() {
        let bad_string = "I \\ have * a _ very ~ bad ` string | here >".to_string();
        let escaped_string = escape_string(&bad_string);

        assert_eq!(
            escaped_string,
            "I \\\\ have \\* a \\_ very \\~ bad \\` string \\| here \\>"
        );
    }

    #[test]
    fn slice_string_exact() {
        let long_string = "殺してください".to_string();
        let cut_string = limit_string_length(&long_string, 7);

        assert_eq!(cut_string, "殺してください");
    }

    #[test]
    fn slice_string_longer() {
        let long_string = "殺してください".to_string();
        let cut_string = limit_string_length(&long_string, 4);

        assert_eq!(cut_string, "殺して…");
    }

    #[test]
    fn slice_string_shorter() {
        let long_string = "殺してください".to_string();
        let cut_string = limit_string_length(&long_string, 14);

        assert_eq!(cut_string, "殺してください");
    }

    #[test]
    fn slice_string_one() {
        let long_string = "少女が好きな".to_string();
        let cut_string = limit_string_length(&long_string, 1);

        assert_eq!(cut_string, "少");
    }

    #[test]
    fn slice_overlapping_unicode() {
        let long_string = "หิวข้าว".to_string();
        let cut_string = limit_string_length(&long_string, 4);

        assert_eq!(cut_string, "หิวข้…");
    }

    #[test]
    fn slice_empty_string() {
        let long_string = "".to_string();
        let cut_string = limit_string_length(&long_string, 4);

        assert_eq!(cut_string, "");
    }
}
