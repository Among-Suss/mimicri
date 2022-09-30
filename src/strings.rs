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

/// Slices the string from zero to width, and rounds to the nearest code point.
/// Does not account for unicode size, as unicode characters tend to be larger.
pub fn limit_string_length(text: &String, width: usize) -> String {
    if text.len() <= width {
        return text.clone();
    }

    let mut previous_code_point = 0;
    let mut previous_previous_code_point = 0;

    for code_point in text.char_indices().map(|(i, _)| i).into_iter() {
        if code_point > width {
            return text.clone()[0..previous_previous_code_point].to_string() + "…";
        }

        previous_previous_code_point = previous_code_point;
        previous_code_point = code_point;
    }

    return text.clone();
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
        let long_string = "I want to die now".to_string();
        let cut_string = limit_string_length(&long_string, 17);

        assert_eq!(cut_string, "I want to die now");
    }

    #[test]
    fn slice_string_longer() {
        let long_string = "I want to die now".to_string();
        let cut_string = limit_string_length(&long_string, 14);

        assert_eq!(cut_string, "I want to die…");
    }

    #[test]
    fn slice_string_shorter() {
        let long_string = "I want to die".to_string();
        let cut_string = limit_string_length(&long_string, 17);

        assert_eq!(cut_string, "I want to die");
    }

    #[test]
    /// As long as it doesn't panic it's fine
    fn slice_string_unicode() {
        let long_string = "人生は意味がない".to_string();
        let cut_string = limit_string_length(&long_string, 7);

        assert_eq!(cut_string, "人…");
    }

    #[test]
    fn slice_string_unicode_2() {
        let long_string = "人生は意味がない".to_string();
        let cut_string = limit_string_length(&long_string, 10);

        assert_eq!(cut_string, "人生…");
    }

    #[test]
    fn slice_string_unicode_longer() {
        let long_string = "人生は意味がない".to_string();
        let cut_string = limit_string_length(&long_string, 25);

        assert_eq!(cut_string, "人生は意味がない");
    }

    #[test]
    /// Not really expected behavior, but I don't think anyone is slicing string with 1 anyway
    fn slice_string_one() {
        let long_string = "I want to die".to_string();
        let cut_string = limit_string_length(&long_string, 1);

        assert_eq!(cut_string, "…");
    }
}
