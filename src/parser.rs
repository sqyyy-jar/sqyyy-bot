use std::{fmt::Display, iter::Peekable, str::Chars};

use emulator::emulator::{and, input, not, or, Component};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidNumber { text: String },
    UnexpectedToken { token: Token },
    UnexpectedRightParen,
    UnknownFunction { name: String },
    UnexpectedComma,
    UnexpectedTokensAfterExpr,
    InvalidParentheses,
    EmptyExpression,
    UnexpectedEndOfSource,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidNumber { text } => {
                f.write_fmt(format_args!("{text:?} is not a valid number"))
            }
            Error::UnexpectedToken { token } => {
                f.write_fmt(format_args!("{token:?} was not expected"))
            }
            Error::UnexpectedRightParen => f.write_str("Invalid closing parenthesis in code"),
            Error::UnknownFunction { name } => {
                f.write_fmt(format_args!("The function {name:?} is unknown"))
            }
            Error::UnexpectedComma => f.write_str("Unexpected comma in code"),
            Error::UnexpectedTokensAfterExpr => f.write_str("Unexpected tokens after expression"),
            Error::InvalidParentheses => f.write_str("Invalid parentheses in code"),
            Error::EmptyExpression => f.write_str("The expression cannot be empty"),
            Error::UnexpectedEndOfSource => f.write_str("Unexpected end of code"),
        }
    }
}

struct Tokenizer<'a> {
    src: Peekable<Chars<'a>>,
    tokens: Vec<Token>,
}

impl Tokenizer<'_> {
    fn has_next(&mut self) -> bool {
        self.src.peek().is_some()
    }

    fn peek(&mut self) -> char {
        *self.src.peek().unwrap()
    }

    fn eat(&mut self) {
        self.src.next().unwrap();
    }
}

enum GroupType<'a> {
    Root,
    Call { call_name: &'a str, inverted: bool },
}

#[derive(Clone, Debug)]
pub enum Token {
    LeftParen,
    RightParen,
    Not,
    Comma,
    Input { index: usize },
    Identifier { value: String },
}

pub fn tokenize(code: &str) -> Result<Vec<Token>> {
    let mut ctx = Tokenizer {
        src: code.chars().peekable(),
        tokens: Vec::new(),
    };
    let mut buffer = String::new();
    while ctx.has_next() {
        let c = ctx.peek();
        match c {
            '(' => {
                if !buffer.is_empty() {
                    ctx.tokens.push(Token::Identifier { value: buffer });
                    buffer = String::new();
                }
                ctx.eat();
                ctx.tokens.push(Token::LeftParen);
            }
            ')' => {
                if !buffer.is_empty() {
                    ctx.tokens.push(Token::Identifier { value: buffer });
                    buffer = String::new();
                }
                ctx.eat();
                ctx.tokens.push(Token::RightParen);
            }
            '!' => {
                if !buffer.is_empty() {
                    ctx.tokens.push(Token::Identifier { value: buffer });
                    buffer = String::new();
                }
                ctx.eat();
                ctx.tokens.push(Token::Not);
            }
            ',' => {
                if !buffer.is_empty() {
                    ctx.tokens.push(Token::Identifier { value: buffer });
                    buffer = String::new();
                }
                ctx.eat();
                ctx.tokens.push(Token::Comma);
            }
            '.' => {
                if !buffer.is_empty() {
                    ctx.tokens.push(Token::Identifier { value: buffer });
                    buffer = String::new();
                }
                ctx.eat();
                let index = parse_number(&mut ctx)?;
                ctx.tokens.push(Token::Input { index });
            }
            c if c.is_whitespace() => {
                if !buffer.is_empty() {
                    ctx.tokens.push(Token::Identifier { value: buffer });
                    buffer = String::new();
                }
                ctx.eat();
            }
            _ => {
                ctx.eat();
                buffer.push(c);
            }
        }
    }
    if !buffer.is_empty() {
        ctx.tokens.push(Token::Identifier { value: buffer });
    }
    Ok(ctx.tokens)
}

pub fn parse(tokens: &[Token]) -> Result<(usize, Component)> {
    let mut tokens = tokens.iter().peekable();
    let mut stack = vec![(GroupType::Root, Vec::with_capacity(1))];
    let mut inverted = false;
    let mut expect_next = true;
    let mut max_input = 0;
    while let Some(token) = tokens.next() {
        match token {
            Token::LeftParen => {
                return Err(Error::UnexpectedToken {
                    token: token.clone(),
                });
            }
            Token::RightParen => {
                if expect_next || stack.len() == 1 {
                    return Err(Error::UnexpectedRightParen);
                }
                let top = stack.pop().unwrap();
                match top.0 {
                    GroupType::Call {
                        call_name,
                        inverted,
                    } => match call_name {
                        "and" => {
                            stack.last_mut().unwrap().1.push(if inverted {
                                not(and(top.1))
                            } else {
                                and(top.1)
                            });
                        }
                        "or" => {
                            stack.last_mut().unwrap().1.push(if inverted {
                                not(or(top.1))
                            } else {
                                or(top.1)
                            });
                        }
                        _ => {
                            return Err(Error::UnknownFunction {
                                name: call_name.to_string(),
                            });
                        }
                    },
                    _ => unreachable!(),
                }
                if stack.len() == 1 && tokens.peek().is_some() {
                    return Err(Error::UnexpectedTokensAfterExpr);
                    // will break either way
                }
            }
            Token::Not => {
                inverted = !inverted;
            }
            Token::Comma => {
                if stack.len() == 1 || expect_next {
                    return Err(Error::UnexpectedComma);
                }
                expect_next = true;
            }
            Token::Input { index } => {
                if !expect_next {
                    return Err(Error::UnexpectedToken {
                        token: token.clone(),
                    });
                }
                expect_next = false;
                if *index >= max_input {
                    max_input = index + 1;
                }
                if stack.len() == 1 {
                    stack.last_mut().unwrap().1.push(if inverted {
                        not(input(*index))
                    } else {
                        input(*index)
                    });
                    if tokens.peek().is_some() {
                        return Err(Error::UnexpectedTokensAfterExpr);
                    }
                    break;
                }
                stack.last_mut().unwrap().1.push(if inverted {
                    not(input(*index))
                } else {
                    input(*index)
                });
                inverted = false;
            }
            Token::Identifier { value } => {
                if !expect_next {
                    return Err(Error::UnexpectedToken {
                        token: token.clone(),
                    });
                }
                expect_next = true;
                let next = tokens.next();
                if next.is_none() {
                    return Err(Error::UnexpectedEndOfSource);
                }
                let Token::LeftParen = next.unwrap() else {
                    return Err(Error::UnexpectedToken { token: token.clone() });
                };
                stack.push((
                    GroupType::Call {
                        call_name: value,
                        inverted,
                    },
                    Vec::with_capacity(2),
                ));
                inverted = false;
            }
        }
    }
    if stack.len() != 1 {
        return Err(Error::InvalidParentheses);
    }
    if stack.last().unwrap().1.is_empty() {
        return Err(Error::EmptyExpression);
    }
    Ok((max_input, stack.pop().unwrap().1.pop().unwrap()))
}

fn parse_number(ctx: &mut Tokenizer) -> Result<usize> {
    let mut buffer = String::new();
    while ctx.has_next() {
        let c = ctx.peek();
        if !c.is_ascii_digit() {
            break;
        }
        ctx.eat();
        buffer.push(c);
    }
    let Ok(num) = buffer.parse() else {
        return Err(Error::InvalidNumber { text: buffer });
    };
    Ok(num)
}
