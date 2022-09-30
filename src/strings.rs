static SENSITIVE_CHARACTERS: [&str; 7] = ["\\", "*", "_", "~", "`", "|", ">"];

///
///
/// see: https://github.com/discord-net/Discord.Net/blob/265da99619a775d23b24326648fe4220bc6beeae/src/Discord.Net.Core/Format.cs#L36
pub fn escape_string(mut text: String) {
    for i in 0..SENSITIVE_CHARACTERS.len() {
        text = text.replace(
            SENSITIVE_CHARACTERS[i],
            format!("\\{}", SENSITIVE_CHARACTERS[i]).as_str(),
        );
    }
}
