#![deny(warnings)]

use crate::helpers;
use crate::scanner::Scanner;

pub struct EbnfTokenizer<I: Iterator<Item=char>> {
    input: Scanner<I>,
    lookahead: Vec<String>,
}

impl<I: Iterator<Item=char>> EbnfTokenizer<I> {
    pub fn new(source: I) -> Self {
        EbnfTokenizer{input: Scanner::new(source), lookahead: Vec::new()}
    }

    pub fn scanner(source: I) -> Scanner<Self> {
        Scanner::new(Self::new(source))
    }
}

impl<I: Iterator<Item=char>> Iterator for EbnfTokenizer<I> {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        // used for accumulating string parts
        if !self.lookahead.is_empty() {
            return self.lookahead.pop();
        }
        let mut s = &mut self.input;
        s.ignore_ws();
        // discard comments starting with '#' until new-line
        if s.accept_char('#') {
            while let Some(nl) = s.next() {
                if nl == '\n' {
                    s.ignore();
                    // discard comment and allow more by restarting
                    return self.next();
                }
            }
        }
        if s.accept_any_char("[]{}()|;").is_some() {
            return Some(s.extract_string());
        }
        let backtrack = s.pos();
        if s.accept_char(':') {
            if s.accept_char('=') {
                return Some(s.extract_string());
            }
            s.set_pos(backtrack);
        }
        let backtrack = s.pos();
        if let Some(q) = s.accept_any_char("\"'") {
            while let Some(n) = s.next() {
                if n == q {
                    // store closing quote
                    self.lookahead.push(n.to_string());
                    // store string content
                    let v = s.extract_string();
                    self.lookahead.push(v[1..v.len()-1].to_string());
                    // return opening quote
                    return Some(q.to_string());
                }
            }
            s.set_pos(backtrack);
        }
        let backtrack = s.pos();
        s.accept_char('@');
        // NOTE: scan_identifier limits the valid options
        if let Some(id) = helpers::scan_identifier(&mut s) {
            return Some(id);
        }
        // backtrack possible '@'
        s.set_pos(backtrack);
        None
    }
}
