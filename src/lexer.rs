#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Let,
    If,
    Else,
    Fn,
    Return,
    Loop,
    While,
    Break,
    Continue,
    Not,
    And,
    Or,
    True,
    False,
    Colon,
    Assign,
    Arrow,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Amp,
    Pipe,
    Tilde,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,
    Comma,
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    Ident(String),
}

pub fn lex(source: &str) -> Result<Vec<Token>, String> {
    let mut chars = source.chars().peekable();
    let mut tokens = Vec::new();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '/' => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    chars.next();
                    while let Some(&next) = chars.peek() {
                        if next == '\n' {
                            break;
                        }
                        chars.next();
                    }
                } else {
                    tokens.push(Token::Slash);
                }
            }
            ':' => {
                chars.next();
                tokens.push(Token::Colon);
            }
            '=' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Eq);
                } else {
                    tokens.push(Token::Assign);
                }
            }
            '!' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Ne);
                } else {
                    return Err("Unexpected character '!'".into());
                }
            }
            '<' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Le);
                } else {
                    tokens.push(Token::Lt);
                }
            }
            '>' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Ge);
                } else {
                    tokens.push(Token::Gt);
                }
            }
            '+' => {
                chars.next();
                tokens.push(Token::Plus);
            }
            '-' => {
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::Arrow);
                } else {
                    tokens.push(Token::Minus);
                }
            }
            '*' => {
                chars.next();
                tokens.push(Token::Star);
            }
            '&' => {
                chars.next();
                tokens.push(Token::Amp);
            }
            '|' => {
                chars.next();
                tokens.push(Token::Pipe);
            }
            '~' => {
                chars.next();
                tokens.push(Token::Tilde);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '{' => {
                chars.next();
                tokens.push(Token::LBrace);
            }
            '}' => {
                chars.next();
                tokens.push(Token::RBrace);
            }
            ';' => {
                chars.next();
                tokens.push(Token::Semicolon);
            }
            ',' => {
                chars.next();
                tokens.push(Token::Comma);
            }
            '"' => {
                chars.next();
                let mut value = String::new();
                while let Some(ch) = chars.next() {
                    match ch {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some('n') => value.push('\n'),
                            Some('t') => value.push('\t'),
                            Some('r') => value.push('\r'),
                            Some('\\') => value.push('\\'),
                            Some('"') => value.push('"'),
                            Some('0') => value.push('\0'),
                            Some(other) => {
                                value.push('\\');
                                value.push(other);
                            }
                            None => return Err("Unterminated string literal".into()),
                        },
                        other => value.push(other),
                    }
                }
                tokens.push(Token::StringLiteral(value));
            }
            c if c.is_ascii_digit() => {
                let mut lexeme = String::new();
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_digit()
                        || next == '_'
                        || next == '.'
                        || next == 'e'
                        || next == 'E'
                        || next == '+'
                        || next == '-'
                    {
                        lexeme.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let cleaned: String = lexeme.chars().filter(|c| *c != '_').collect();
                if cleaned.contains('.') || cleaned.contains('e') || cleaned.contains('E') {
                    match cleaned.parse::<f64>() {
                        Ok(value) => tokens.push(Token::FloatLiteral(value)),
                        Err(_) => return Err(format!("Invalid float literal '{}", lexeme)),
                    }
                } else {
                    match cleaned.parse::<i64>() {
                        Ok(value) => tokens.push(Token::IntLiteral(value)),
                        Err(_) => return Err(format!("Invalid integer literal '{}", lexeme)),
                    }
                }
            }
            c if is_identifier_start(c) => {
                let mut ident = String::new();
                ident.push(c);
                chars.next();
                while let Some(&next) = chars.peek() {
                    if is_identifier_continue(next) {
                        ident.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let keyword = match ident.as_str() {
                    "let" => Token::Let,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "fn" => Token::Fn,
                    "return" => Token::Return,
                    "loop" => Token::Loop,
                    "while" => Token::While,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "not" => Token::Not,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "true" => Token::True,
                    "false" => Token::False,
                    other => Token::Ident(other.to_string()),
                };
                tokens.push(keyword);
            }
            other => {
                return Err(format!("Unexpected character '{}'", other));
            }
        }
    }

    if std::env::var("DEBUG_LEX").is_ok() {
        eprintln!("tokens={:?}", tokens);
    }
    Ok(tokens)
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
