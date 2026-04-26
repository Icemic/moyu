pub(crate) fn is_wildcard(name: &str) -> bool {
    name.contains('*')
}

pub(crate) fn wildcard_match(pattern: &str, name: &str) -> bool {
    if !is_wildcard(pattern) {
        return pattern == name;
    }

    let pattern = pattern.as_bytes();
    let name = name.as_bytes();
    let mut pattern_index = 0;
    let mut name_index = 0;
    let mut wildcard_index = None;
    let mut wildcard_match_index = 0;

    while name_index < name.len() {
        if pattern_index < pattern.len() && pattern[pattern_index] == name[name_index] {
            pattern_index += 1;
            name_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            wildcard_index = Some(pattern_index);
            pattern_index += 1;
            wildcard_match_index = name_index;
        } else if let Some(star_index) = wildcard_index {
            pattern_index = star_index + 1;
            wildcard_match_index += 1;
            name_index = wildcard_match_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::{is_wildcard, wildcard_match};

    #[test]
    fn wildcard_detects_star_only() {
        assert!(is_wildcard("voice:*"));
        assert!(!is_wildcard("voice:alice"));
        assert!(!is_wildcard("voice:?"));
    }

    #[test]
    fn wildcard_exact_match_stays_literal() {
        assert!(wildcard_match("bgm", "bgm"));
        assert!(!wildcard_match("bgm", "bgm:main"));
    }

    #[test]
    fn wildcard_matches_prefix_pattern() {
        assert!(wildcard_match("voice:*", "voice:alice"));
        assert!(wildcard_match("voice:*", "voice:"));
        assert!(!wildcard_match("voice:*", "voice_alice"));
    }

    #[test]
    fn wildcard_matches_middle_segments() {
        assert!(wildcard_match("voice:*:line", "voice:alice:line"));
        assert!(wildcard_match("voice:*:line", "voice:alice:extra:line"));
        assert!(!wildcard_match("voice:*:line", "voice:alice"));
    }

    #[test]
    fn wildcard_matches_multiple_stars() {
        assert!(wildcard_match("*:battle:*", "bgm:battle:intro"));
        assert!(wildcard_match("**", "voice:alice"));
    }
}
