use unicode_categories::UnicodeCategories;
use unscanny::Scanner;

use crate::Range;

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
            return Token { kind: NormalTokenKind::EOF, range };
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
                    NormalTokenKind::EOF
                }
                _ => NormalTokenKind::Whitespace,
            }
        } else if c == NEWLINE {
            self.s.eat();
            NormalTokenKind::Newline
        } else if c.is_punctuation() {
            self.s.eat_while(c);
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

#[derive(Debug, PartialEq)]
pub struct Token<K: TokenKind> {
    pub kind: K,
    pub range: Range,
}
#[derive(Debug, PartialEq)]
pub enum NormalTokenKind {
    Word,
    Special(char),
    Whitespace,
    /// EOL character including trailing whitespaces
    Newline,
    EOF,
}
pub trait TokenKind {
    const END: Self;
}
impl TokenKind for NormalTokenKind {
    const END: Self = Self::EOF;
}
impl TokenKind for ArgumentTokenKind {
    const END: Self = Self::EOF;
}
impl TokenKind for RangedTagTokenKind {
    const END: Self = Self::EOF;
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
        matches!(self.kind, Whitespace | Newline | EOF)
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
        matches!(self.kind, Newline | EOF)
    }
    /// match Word or '-'
    pub fn is_identifier(&self) -> bool {
        use NormalTokenKind::*;
        matches!(self.kind, Word | Special('-'))
    }
}
