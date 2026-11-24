pub(super) fn literal_pattern(keyword: &str) -> String {
    let mut pattern = String::from("%");
    for character in keyword.chars() {
        if matches!(character, '!' | '%' | '_') {
            pattern.push('!');
        }
        pattern.push(character);
    }
    pattern.push('%');
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_search_escapes_only_the_explicit_pattern_metacharacters() {
        assert_eq!(literal_pattern(""), "%%");
        assert_eq!(literal_pattern("书 & ABC"), "%书 & ABC%");
        assert_eq!(literal_pattern("!%_\\"), "%!!!%!_\\%");
    }
}
