static SENSITIVE_CHARACTERS: [&str; 7] = ["\\", "*", "_", "~", "`", "|", ">"];

///
///
/// see: https://github.com/discord-net/Discord.Net/blob/265da99619a775d23b24326648fe4220bc6beeae/src/Discord.Net.Core/Format.cs#L36
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_string() {
        let bad_string = "I \\ have * a _ very ~ bad ` string | here >".to_string();

        assert_eq!(
            escape_string(&bad_string),
            "I \\\\ have \\* a \\_ very \\~ bad \\` string \\| here \\>"
        );
    }
}
