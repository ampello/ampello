// SPDX-License-Identifier: GPL-3.0-or-later
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryMode {
    Word,

    Anywhere,
}

impl BoundaryMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "anywhere" => BoundaryMode::Anywhere,
            _ => BoundaryMode::Word,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BoundaryMode::Word => "word",
            BoundaryMode::Anywhere => "anywhere",
        }
    }
}

pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn is_terminator(c: char) -> bool {
    !is_word_char(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_characters_span_unicode() {
        for c in ['a', 'Z', '0', '_', 'é', 'ß', 'あ', '漢'] {
            assert!(is_word_char(c), "{c} should be a word character");
        }
    }

    #[test]
    fn everything_else_terminates() {
        for c in [
            ' ', '\t', '\n', '.', ',', '!', ':', ';', '(', '-', '/', '\u{a0}',
        ] {
            assert!(is_terminator(c), "{c:?} should terminate a trigger");
        }
    }

    #[test]
    fn mode_parsing_defaults_to_word() {
        assert_eq!(BoundaryMode::parse("anywhere"), BoundaryMode::Anywhere);
        assert_eq!(BoundaryMode::parse("word"), BoundaryMode::Word);
        assert_eq!(BoundaryMode::parse("nonsense"), BoundaryMode::Word);
    }
}
