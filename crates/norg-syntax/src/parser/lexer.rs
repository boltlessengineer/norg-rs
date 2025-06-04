use unicode_categories::UnicodeCategories;
use unscanny::Scanner;

use crate::{node::SyntaxKind, Range};

pub struct Lexer<'s> {
    s: Scanner<'s>,
}

// TODO: replace these with proper patterns
const SPACE: char = ' ';
const NEWLINE: char = '\n';

impl<'s> Lexer<'s> {
    pub fn new(text: &'s str) -> Self {
        Self {
            s: Scanner::new(text),
        }
    }

    pub fn next(&mut self) -> Token<NormalTokenKind> {
        let start = self.s.cursor();
        let Some(c) = self.s.peek() else {
            let range = Range::new(start, self.s.cursor());
            return Token { kind: NormalTokenKind::End, range };
        };
        let kind = if c == SPACE {
            self.s.eat_while(SPACE);
            match self.s.peek() {
                Some(NEWLINE) => {
                    self.s.eat();
                    NormalTokenKind::Newline
                }
                None => {
                    self.s.eat();
                    NormalTokenKind::End
                }
                _ => NormalTokenKind::Whitespace,
            }
        } else if c == NEWLINE {
            self.s.eat();
            NormalTokenKind::Newline
        } else if c.is_punctuation() {
            self.s.eat();
            NormalTokenKind::Special(c)
        } else {
            self.s
                .eat_while(|c: char| !c.is_whitespace() && !c.is_punctuation());
            NormalTokenKind::Word
        };
        let range = Range::new(start, self.s.cursor());
        Token { kind, range }
    }
    pub fn peek(&mut self) -> Token<NormalTokenKind> {
        let origin = self.s.cursor();
        let token = self.next();
        self.s.jump(origin);
        token
    }
    pub fn next_arg(&mut self) -> Token<ArgumentTokenKind> {
        let start = self.s.cursor();
        if self.s.eat_if(';') {
            return Token {
                kind: ArgumentTokenKind::Delimiter,
                range: Range::new(start, self.s.cursor()),
            };
        }
        while !matches!(self.s.peek(), Some(';' | NEWLINE) | None) {
            let t = self.s.eat();
            // try eat one more character after backslash
            if t == Some('\\') && self.s.peek().is_some() {
                self.s.eat();
            }
        }
        let range = Range::new(start, self.s.cursor());
        Token {
            kind: if range.is_empty() {
                ArgumentTokenKind::EOF
            } else {
                ArgumentTokenKind::Argument
            },
            range,
        }
    }
    // TODO: should accept starting column to trim correct amount of preceding whitespaces
    pub fn next_ranged_tag(&mut self, level: usize) -> Token<RangedTagTokenKind> {
        // // skip preceding whitespaces
        // while self.s.peek() == Some(SPACE) {
        //     self.s.eat();
        // }
        let start = self.s.cursor();
        // advance remaining preceding whitespaces to find ending modifier
        self.s.eat_while(SPACE);
        let start_nonblank = self.s.cursor();
        if self
            .s
            .eat_if(format!("{}end\n", "@".repeat(level)).as_str())
        {
            return Token {
                kind: RangedTagTokenKind::EndModifier,
                range: Range::new(start_nonblank, self.s.cursor()),
            };
        }
        self.eat_line();
        let range = Range::new(start, self.s.cursor());
        Token {
            kind: if range.is_empty() {
                RangedTagTokenKind::EOF
            } else {
                RangedTagTokenKind::VerbatimLine
            },
            range,
        }
    }
    /// advance remaining line (including eol character)
    pub fn eat_line(&mut self) -> Range {
        let start = self.s.cursor();
        self.s.eat_until(NEWLINE);
        self.s.eat();
        Range::new(start, self.s.cursor())
    }
}

pub struct Lexer2<'s> {
    s: Scanner<'s>,
}

impl<'s> Lexer2<'s> {
    pub fn new(text: &'s str) -> Self {
        Self {
            s: Scanner::new(text),
        }
    }
    pub fn cursor(&self) -> usize {
        self.s.cursor()
    }
    pub fn peek(&self) -> Token<NormalTokenKind> {
        let mut s = self.s;
        Self::lex_at(&mut s)
    }
    pub fn jump(&mut self, pos: usize) {
        self.s.jump(pos);
    }
    pub fn eat(&mut self) -> Token<NormalTokenKind> {
        Self::lex_at(&mut self.s)
    }
    fn lex_at(s: &mut Scanner) -> Token<NormalTokenKind> {
        let start = s.cursor();
        let Some(ch) = s.eat() else {
            return Token { kind: NormalTokenKind::End, range: Range::new(s.cursor(), s.cursor()) };
        };
        let kind = if ch == SPACE {
            s.eat_while(SPACE);
            match s.peek() {
                Some(NEWLINE) => {
                    s.eat();
                    NormalTokenKind::Newline
                }
                None => {
                    NormalTokenKind::End
                }
                _ => NormalTokenKind::Whitespace,
            }
        } else if ch == NEWLINE {
            NormalTokenKind::Special(ch)
        } else if ch.is_ascii_punctuation() { // TODO: change to proper punctuation detection
            NormalTokenKind::Special(ch)
        } else {
            s.eat_while(|c: char| !c.is_whitespace() && !c.is_punctuation());
            NormalTokenKind::Word
        };
        let range = Range::new(start, s.cursor());
        Token { kind, range }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token<K> {
    pub kind: K,
    pub range: Range,
}
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum NormalTokenKind {
    Word,
    Special(char),
    Whitespace,
    /// EOL character including trailing whitespaces
    Newline,
    End,
}
impl NormalTokenKind {
    pub fn to_syntax_kind(&self) -> SyntaxKind {
        match self {
            Self::Word => SyntaxKind::Word,
            Self::Special(ch) => SyntaxKind::Special(*ch),
            Self::Whitespace => SyntaxKind::Whitespace,
            Self::Newline => SyntaxKind::SoftBreak,
            Self::End => SyntaxKind::End,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ArgumentTokenKind {
    Argument,
    Delimiter,
    EOF,
}

#[derive(Debug, PartialEq)]
pub enum RangedTagTokenKind {
    VerbatimLine,
    EndModifier,
    EOF,
}

impl Token<NormalTokenKind> {
    pub fn len(&self) -> usize {
        self.range.end - self.range.start
    }
    pub fn is_char(&self, c: char) -> bool {
        self.len() == c.len_utf8() && self.kind == NormalTokenKind::Special(c)
    }
    /// match all whitespace characters includine EOF
    pub fn is_whitespace(&self) -> bool {
        use NormalTokenKind::*;
        matches!(self.kind, Whitespace | Newline | End)
    }
    pub fn is_word(&self) -> bool {
        use NormalTokenKind::*;
        matches!(self.kind, Word)
    }
    pub fn is_punctuation(&self) -> bool {
        use NormalTokenKind::*;
        matches!(self.kind, Special(_))
    }
    /// match Newline or EOF
    pub fn is_eol(&self) -> bool {
        use NormalTokenKind::*;
        matches!(self.kind, Newline | End)
    }
    /// match Word or '-'
    pub fn is_identifier(&self) -> bool {
        use NormalTokenKind::*;
        matches!(self.kind, Word | Special('-'))
    }
    pub fn to_syntax_token(&self) -> Token<SyntaxKind> {
        Token { kind: self.kind.to_syntax_kind(), range: self.range }
    }
}
