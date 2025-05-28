use unicode_categories::UnicodeCategories as _;
use unscanny::Scanner;

use crate::{node::SyntaxKind, Range};

pub fn lex_next(s: &mut Scanner) -> Option<(LinkTokenKind, Range)> {
    if s.at('}') {
        return None;
    }
    let start = s.cursor();
    let ch = s.eat()?;
    let kind = match ch {
        ':' => LinkTokenKind::Delimiter,
        '$' => LinkTokenKind::WorkspacePrefix,
        '/' => LinkTokenKind::PathSep,
        '*' => {
            s.eat_while(ch);
            LinkTokenKind::HeadingPrefix
        }
        '?' => LinkTokenKind::WikiHeadingPrefix,
        '\\' if s.eat_if(char_can_escaped) => LinkTokenKind::Escaped,
        '\\' => LinkTokenKind::BackSlash,
        _ if ch.is_whitespace() => {
            s.eat_whitespace();
            LinkTokenKind::Whitespace
        }
        _ => {
            s.eat_while(|c: char| match c {
                ':' | '$' | '/' | '*' | '?' | '\\' => false,
                _ if c.is_whitespace() => false,
                _ => true,
            });
            LinkTokenKind::Text
        }
    };
    Some((kind, Range::new(start, s.cursor())))
}

fn char_can_escaped(ch: char) -> bool {
    return ch.is_punctuation() || ch.is_whitespace();
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LinkTokenKind {
    /// ':'
    Delimiter,
    /// '$'
    WorkspacePrefix,
    /// `/`
    PathSep,
    /// repeated `*` character
    HeadingPrefix,
    /// `?`
    WikiHeadingPrefix,
    /// `\*`
    Escaped,
    /// backslash character that isn't used as escaped modifier
    /// e.g. `\a` or `\`(end of input)
    BackSlash,
    /// any whitespaces between tokens including line break
    Whitespace,
    /// any general text as fallback
    Text,
}

impl LinkTokenKind {
    pub fn to_syntax(&self) -> SyntaxKind {
        match self {
            Self::BackSlash => SyntaxKind::Punctuation,
            Self::Whitespace => SyntaxKind::Whitespace,
            Self::Text => SyntaxKind::Word,
            Self::Escaped => todo!("handle escaped"),

            Self::Delimiter => todo!(),
            Self::WorkspacePrefix => todo!(),
            Self::PathSep => todo!(),
            Self::HeadingPrefix => todo!(),
            Self::WikiHeadingPrefix => todo!(),
        }
    }
}
