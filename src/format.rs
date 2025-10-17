use std::collections::HashMap;

pub fn clean_text(text: &str) -> String {
    let mut replacements: HashMap<char, char> = HashMap::new();
    replacements.insert('‘', '\'');
    replacements.insert('’', '\'');
    replacements.insert('‚', '\'');
    replacements.insert('‛', '\'');
    replacements.insert('ʼ', '\'');
    replacements.insert('ʹ', '\'');
    replacements.insert('ʻ', '\'');
    replacements.insert('“', '"');
    replacements.insert('”', '"');
    replacements.insert('„', '"');
    replacements.insert('‟', '"');
    replacements.insert('ʺ', '"');
    replacements.insert('«', '"');
    replacements.insert('»', '"');
    replacements.insert('–', '-');
    replacements.insert('—', '-');
    replacements.insert('―', '-');
    replacements.insert('﹣', '-');
    replacements.insert('－', '-');
    replacements.insert('∕', '/');
    replacements.insert('／', '/');
    replacements.insert('⧸', '/');
    replacements.insert('＼', '\\');

    // Normalize ellipsis to three dots first
    let text = text.replace('…', "...");

    // Map characters using the replacements table, then filter out control/non-graphic chars
    let processed_text: String = text
        .chars()
        .map(|c| replacements.get(&c).cloned().unwrap_or(c))
        .filter(|&c| (c.is_ascii_graphic() || c.is_ascii_whitespace()) && c != '\u{000C}')
        .collect();

    // Split on any whitespace and join tokens with newlines for one-token-per-line output
    let tokens: Vec<&str> = processed_text.split_whitespace().collect();
    tokens.join("\n")
}
