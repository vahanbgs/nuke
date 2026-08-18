pub(crate) fn escape(out: &mut String, text: &str) -> Result<(), char> {
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            forbidden if is_forbidden(forbidden) => return Err(forbidden),
            other => out.push(other),
        }
    }
    Ok(())
}

fn is_forbidden(character: char) -> bool {
    matches!(
        character,
        '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}'
    )
}
