use crate::intermediate::{ Token, TokenValue, Position, TokenList, CompileErrorList };

use std::iter::Peekable;
use std::str::Chars;

/// A macro to match a token value with a value from a list of options.
///
/// This macro takes a token value and a list of options in the form of
/// `key => value`. It will return `Some(value)` if the token matches `key`
/// and `None` otherwise.
macro_rules! match_token {
    ($token: ident, $($key: expr => $value: expr), *) => {
        match $token {
            $( $key => return Some($value), )*
            _ => None
        }
    }
}


struct LexState<'a> {
    source: Peekable<Chars<'a>>,
    position: Position, /// The current position in the source
    current: Option<char>
}

impl<'a> LexState<'a> {
    fn skip_spaces_and_comments(&mut self) {
        while self.current.map_or(false, |c| is_space(c) || is_comment_prefix(c)) {
            if self.current.map_or(false, is_space) {
                self.next_character();
            } else if self.current.map_or(false, is_comment_prefix) {
                self.skip_comment();
            }
        }
    }    
    fn skip_comment(&mut self) {
        while self.next_character().is_some() && self.current != Some('\n') {};
    }

    fn next_character(&mut self) -> Option<char> {
        if let Some(character) = self.source.next() {
            self.current = Some(character);
            self.position.column += 1;

            if character == '\n' {
                self.position.line += 1;
                self.position.column = 0;
            };

            Some(character)
        } else {
            self.current = None;
            None
        }
    }

    fn lex_string(&mut self) -> Token {
        let position = self.position;
        let mut value = String::new();
        let mut is_escaping = false; // Flag to track if the previous character was a backslash

        while let Some(character) = self.next_character() {
            if is_escaping {
                value.push(match character {
                    '\\' | '\"' => character,
                    't' => '\t',
                    'n' => '\n',
                    'r' => '\r',
                    _ => character,
                });
                is_escaping = false;
            } else if character == '\"' {
                break;
            } else if character == '\\' {
                is_escaping = true;
            } else {
                value.push(character);
            }
        }

        if self.current != Some('\"') {
            return Token::new(TokenValue::Invalid("EOF while parsing string".to_string()), position);
        };

        // we stop at " character, so move to next
        self.next_character();

        Token::new(TokenValue::String(value), position)
    }

    fn lex_number(&mut self) -> Token {
        let position = self.position;
        let mut number_string = String::new();
        let mut is_float = false;

        while let Some(c) = self.current {
            if is_number(c) {
                number_string.push(c);
            } else if c == '.' {
                if is_float || !is_number(self.peek()) {
                    break;
                }
                is_float = true;
                number_string.push(c);
            } else {
                break;
            }
            self.next_character();
        }

        let value = match number_string.parse::<f64>() {
            Ok(num) if is_float => TokenValue::Float(num),
            Ok(num) => TokenValue::Integer(num as i64),
            Err(_) => TokenValue::Invalid(format!("Invalid number '{}'", number_string)),
        };

        Token::new(value, position)
    }

    fn lex_identifier(&mut self) -> Token {
        let mut identifier = String::new();
        let position = self.position;

        loop {
            identifier.push(self.current.expect("Current character should be valid"));

            if self.next_character().is_none() {
                break;
            };

            if !is_identifier(self.current.expect("Current character should be valid")) && !is_number(self.current.expect("Current character should be valid")) {
                break;
            };
        };


        if let Some(keyword) = get_keyword(identifier.as_str()) {
            Token::new(keyword, position)
        } else {
            Token::new(TokenValue::Identifier(identifier), position)
        }
    }

    fn lex_symbol(&mut self) -> Token {
        let position = self.position;

        let symbol_string = self.current.map(|c| c.to_string()).unwrap_or_default();

        self.next_character();

        if let Some(current) = self.current {
            if is_symbol(current) {
                let mut multi_character_symbol_string = symbol_string.clone();
                multi_character_symbol_string.push(current);

                if let Some(symbol) = get_symbol(multi_character_symbol_string.as_str()) {
                    self.next_character();
                    return Token::new(symbol, position);
                }
            }
        }

        Token::new(get_symbol(symbol_string.as_str()).unwrap(), position)
    }
    fn peek(&mut self) -> char {
        self.source.peek().copied().unwrap_or('\0')
    }
}

impl<'a> Iterator for LexState<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_spaces_and_comments();

        let character = self.current?;

        match character {
            '\0' => return None,
            _ if is_identifier(character) => return Some(self.lex_identifier()),
            _ if is_string(character) => return Some(self.lex_string()),
            _ if is_number(character) => return Some(self.lex_number()),
            _ if is_symbol(character) => return Some(self.lex_symbol()),
            _ => {
                self.next_character();
                Some(Token::new(TokenValue::Invalid(format!("Unknown character [{}]", character)), self.position))
            }
        }    
    }
}

// character helpers

fn is_space(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\r' | '\n')
}

fn is_string(character: char) -> bool {
    character == '\"'
}

fn is_number(character: char) -> bool {
    character.is_ascii_digit()
}

fn is_alpha(character: char) -> bool {
    character.is_ascii_alphabetic()
}

fn is_identifier(character: char) -> bool {
    is_alpha(character) || character == '_'
}

fn is_comment_prefix(character: char) -> bool {
    character == '#'
}

fn is_symbol(character: char) -> bool {
    let string = String::from(character);

    get_symbol(string.as_str()).is_some()
}

// token helper functions

fn get_keyword(keyword: &str) -> Option<TokenValue> {
    match_token! {
        keyword,
        "true"          => TokenValue::True,
        "false"         => TokenValue::False,
        "null"          => TokenValue::Null,

        "and"           => TokenValue::And,
        "or"            => TokenValue::Or,
        "not"           => TokenValue::Not,

        "include"       => TokenValue::Include,
        "from"          => TokenValue::From,
        "model"         => TokenValue::Model,
        "function"      => TokenValue::Function,
        "end"           => TokenValue::End,
        "implement"     => TokenValue::Implement,
        "local"         => TokenValue::Local,
        "apply"         => TokenValue::Apply,
        "to"            => TokenValue::To,
        "return"        => TokenValue::Return,
        "public"        => TokenValue::Public,
        "as"            => TokenValue::As,
        "this"          => TokenValue::This,
        "if"            => TokenValue::If,
        "else"          => TokenValue::Else,
        "elseif"        => TokenValue::ElseIf,
        "while"         => TokenValue::While,
        "for"           => TokenValue::For,
        "in"            => TokenValue::In,
        "break"         => TokenValue::Break,

        "rescue"        => TokenValue::Rescue
    }
}

fn get_symbol(symbol: &str) -> Option<TokenValue> {
    match_token! {
        symbol,
        "="     =>  TokenValue::Assign,
        "+"     =>  TokenValue::Plus,
        "-"     =>  TokenValue::Minus,
        "*"     =>  TokenValue::Star,
        "/"     =>  TokenValue::Slash,
        "%"     =>  TokenValue::Percent,
        "!"     =>  TokenValue::Not,
        "("     =>  TokenValue::LeftParentheses,
        ")"     =>  TokenValue::RightParentheses,
        "["     =>  TokenValue::LeftBracket,
        "]"     =>  TokenValue::RightBracket,
        ","     =>  TokenValue::Comma,
        ":"     =>  TokenValue::Colon,
        "&"     =>  TokenValue::BitAnd,
        "|"     =>  TokenValue::BitOr,
        "."     =>  TokenValue::Dot,
        ">"     =>  TokenValue::Greater,
        "<"     =>  TokenValue::Less,

        "=="    =>  TokenValue::Equal,
        "!="    =>  TokenValue::NotEqual,
        "&&"    =>  TokenValue::And,
        "||"    =>  TokenValue::Or,
        ">="    =>  TokenValue::GreaterEqual,
        "<="    =>  TokenValue::LessEqual,
        "+="    =>  TokenValue::PlusAssign,
        "-="    =>  TokenValue::MinusAssign,
        "*="    =>  TokenValue::StarAssign,
        "/="    =>  TokenValue::SlashAssign,
        "%="    =>  TokenValue::PercentAssign
    }
}


// the main lex function
pub fn lex(source: &str) -> Result<TokenList, CompileErrorList> {
    let mut state = LexState {
        source: source.chars().peekable(),
        position: Position::new(1, 0),
        current: Some('\0')
    };

    state.next_character();

    let mut tokens = TokenList::new();

    while let Some(token) = state.next() {
        tokens.push(token);
    }
    
    // Add an Eof token to mark the end of the input stream.
    tokens.push(Token::new(TokenValue::Eof, state.position));

    Ok(tokens)
}


#[cfg(test)]
mod tests {
    use crate::frontend::lexer::get_keyword;
    use crate::frontend::lexer::get_symbol;
    use crate::intermediate::TokenValue;

    #[test]
    fn test_get_keyword() {
        assert_eq!(get_keyword("true"), Some(TokenValue::True));
        assert_eq!(get_keyword("false"), Some(TokenValue::False));
        assert_eq!(get_keyword("null"), Some(TokenValue::Null));

        assert_eq!(get_keyword("and"), Some(TokenValue::And));
        assert_eq!(get_keyword("or"), Some(TokenValue::Or));
        assert_eq!(get_keyword("not"), Some(TokenValue::Not));

        assert_eq!(get_keyword("include"), Some(TokenValue::Include));
        assert_eq!(get_keyword("from"), Some(TokenValue::From));
        assert_eq!(get_keyword("model"), Some(TokenValue::Model));
        assert_eq!(get_keyword("function"), Some(TokenValue::Function));
        assert_eq!(get_keyword("end"), Some(TokenValue::End));
        assert_eq!(get_keyword("implement"), Some(TokenValue::Implement));
        assert_eq!(get_keyword("local"), Some(TokenValue::Local));
        assert_eq!(get_keyword("apply"), Some(TokenValue::Apply));
        assert_eq!(get_keyword("to"), Some(TokenValue::To));
        assert_eq!(get_keyword("return"), Some(TokenValue::Return));
        assert_eq!(get_keyword("public"), Some(TokenValue::Public));
        assert_eq!(get_keyword("as"), Some(TokenValue::As));
        assert_eq!(get_keyword("this"), Some(TokenValue::This));
        assert_eq!(get_keyword("if"), Some(TokenValue::If));
        assert_eq!(get_keyword("else"), Some(TokenValue::Else));
        assert_eq!(get_keyword("elseif"), Some(TokenValue::ElseIf));
        assert_eq!(get_keyword("while"), Some(TokenValue::While));
        assert_eq!(get_keyword("for"), Some(TokenValue::For));
        assert_eq!(get_keyword("in"), Some(TokenValue::In));
        assert_eq!(get_keyword("break"), Some(TokenValue::Break));

        assert_eq!(get_keyword("rescue"), Some(TokenValue::Rescue));
    }

    #[test]
    fn test_get_symbol() {
        assert_eq!(get_symbol("="), Some(TokenValue::Assign));
        assert_eq!(get_symbol("+"), Some(TokenValue::Plus));
        assert_eq!(get_symbol("-"), Some(TokenValue::Minus));
        assert_eq!(get_symbol("*"), Some(TokenValue::Star));
        assert_eq!(get_symbol("/"), Some(TokenValue::Slash));
        assert_eq!(get_symbol("%"), Some(TokenValue::Percent));
        assert_eq!(get_symbol("!"), Some(TokenValue::Not));
        assert_eq!(get_symbol("("), Some(TokenValue::LeftParentheses));
        assert_eq!(get_symbol(")"), Some(TokenValue::RightParentheses));
        assert_eq!(get_symbol("["), Some(TokenValue::LeftBracket));
        assert_eq!(get_symbol("]"), Some(TokenValue::RightBracket));
        assert_eq!(get_symbol(","), Some(TokenValue::Comma));
        assert_eq!(get_symbol(":"), Some(TokenValue::Colon));
        assert_eq!(get_symbol("&"), Some(TokenValue::BitAnd));
        assert_eq!(get_symbol("|"), Some(TokenValue::BitOr));
        assert_eq!(get_symbol("."), Some(TokenValue::Dot));
        assert_eq!(get_symbol(">"), Some(TokenValue::Greater));
        assert_eq!(get_symbol("<"), Some(TokenValue::Less));

        assert_eq!(get_symbol("=="), Some(TokenValue::Equal));
        assert_eq!(get_symbol("!="), Some(TokenValue::NotEqual));
        assert_eq!(get_symbol("&&"), Some(TokenValue::And));
        assert_eq!(get_symbol("||"), Some(TokenValue::Or));
        assert_eq!(get_symbol(">="), Some(TokenValue::GreaterEqual));
        assert_eq!(get_symbol("<="), Some(TokenValue::LessEqual));
        assert_eq!(get_symbol("+="), Some(TokenValue::PlusAssign));
        assert_eq!(get_symbol("-="), Some(TokenValue::MinusAssign));
        assert_eq!(get_symbol("*="), Some(TokenValue::StarAssign));
        assert_eq!(get_symbol("/="), Some(TokenValue::SlashAssign));
        assert_eq!(get_symbol("%="), Some(TokenValue::PercentAssign));
    }
}