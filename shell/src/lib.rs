#![no_std]
#![feature(concat_idents)]

extern crate alloc;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, alphanumeric1, char, none_of, one_of};
use nom::combinator::{map_res, opt, recognize};
use nom::IResult;
use nom::multi::{many0, many1};
use nom::sequence::{delimited, pair, preceded, terminated, tuple};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// #[derive(Debug, Clone, Copy)]
// pub struct ParseLocation {
//     pub line: usize,
//     pub col: usize,
//     pub index: usize,
// }
//
// #[derive(Debug, Clone, Copy)]
// pub enum Span {
//     Location(ParseLocation),
//     Range(ParseLocation, ParseLocation)
// }

#[derive(Debug, Clone)]
pub enum Token {
    Literal(LiteralKind),
    Keyword(Keyword),
    Identifier(String),
    Symbol(Symbol),
    Whitespace
}

#[derive(Debug, Clone)]
pub enum LiteralKind {
    Integer(i64),
    Float(f64),
    String(String)
}

#[derive(Debug, Clone)]
pub enum Keyword { True, False, Let }

#[derive(Debug, Clone)]
pub enum Symbol {
    Comma, Period, Underscore, LParen, RParen, Plus, Minus, Star, Slash, Pipe, Caret, Equals, Semicolon, Colon
}

type TResult = core::result::Result::<Token, &'static str>;

pub fn parse(input: &str) -> IResult<&str, Vec<Token>> {
    fn comma(i: &str)      -> IResult<&str, Token> { map_res(char(','), |_: char| TResult::Ok(Token::Symbol(Symbol::Comma)))(i) }
    fn period(i: &str)     -> IResult<&str, Token> { map_res(char('.'), |_: char| TResult::Ok(Token::Symbol(Symbol::Period)))(i) }
    fn underscore(i: &str) -> IResult<&str, Token> { map_res(char('_'), |_: char| TResult::Ok(Token::Symbol(Symbol::Underscore)))(i) }
    fn lparen(i: &str)     -> IResult<&str, Token> { map_res(char('('), |_: char| TResult::Ok(Token::Symbol(Symbol::LParen)))(i) }
    fn rparen(i: &str)     -> IResult<&str, Token> { map_res(char(')'), |_: char| TResult::Ok(Token::Symbol(Symbol::RParen)))(i) }
    fn plus(i: &str)       -> IResult<&str, Token> { map_res(char('+'), |_: char| TResult::Ok(Token::Symbol(Symbol::Plus)))(i) }
    fn minus(i: &str)      -> IResult<&str, Token> { map_res(char('-'), |_: char| TResult::Ok(Token::Symbol(Symbol::Minus)))(i) }
    fn star(i: &str)       -> IResult<&str, Token> { map_res(char('*'), |_: char| TResult::Ok(Token::Symbol(Symbol::Star)))(i) }
    fn slash(i: &str)      -> IResult<&str, Token> { map_res(char('/'), |_: char| TResult::Ok(Token::Symbol(Symbol::Slash)))(i) }
    fn pipe(i: &str)       -> IResult<&str, Token> { map_res(char('|'), |_: char| TResult::Ok(Token::Symbol(Symbol::Pipe)))(i) }
    fn caret(i: &str)      -> IResult<&str, Token> { map_res(char('^'), |_: char| TResult::Ok(Token::Symbol(Symbol::Caret)))(i) }
    fn equals(i: &str)     -> IResult<&str, Token> { map_res(char('='), |_: char| TResult::Ok(Token::Symbol(Symbol::Equals)))(i) }
    fn colon(i: &str)      -> IResult<&str, Token> { map_res(char(':'), |_: char| TResult::Ok(Token::Symbol(Symbol::Colon)))(i) }
    fn semicolon(i: &str)  -> IResult<&str, Token> { map_res(char(';'), |_: char| TResult::Ok(Token::Symbol(Symbol::Semicolon)))(i) }

    fn symbol(input: &str) -> IResult<&str, Token> {
        alt((comma, period, underscore, lparen, rparen, plus, minus, star, slash, pipe, caret, equals, colon, semicolon))(input)
    }

    fn whitespace(i: &str) -> IResult<&str, Token> {
        map_res(
            many1(alt((char(' '), char('\t'), char('\r'), char('\n')))),
            |_| TResult::Ok(Token::Whitespace)
        )(i)
    }

    pub fn identifier(input: &str) -> IResult<&str, Token> {
        map_res(recognize(pair(
                alt((alpha1, tag("_"))),
                many0(alt((alphanumeric1, tag("_"))))
        )), |s: &str| TResult::Ok(Token::Identifier(s.to_string())))(input)
    }

    fn kw_true(i: &str)  -> IResult<&str, Token> { map_res(tag("true"), |_| TResult::Ok(Token::Keyword(Keyword::True)))(i) }
    fn kw_false(i: &str) -> IResult<&str, Token> { map_res(tag("false"), |_| TResult::Ok(Token::Keyword(Keyword::False)))(i) }
    fn kw_let(i: &str)   -> IResult<&str, Token> { map_res(tag("let"), |_| TResult::Ok(Token::Keyword(Keyword::Let)))(i) }

    fn keyword(input: &str) -> IResult<&str, Token> {
        alt((kw_true, kw_false, kw_let))(input)
    }

    fn boolean(input: &str) -> IResult<&str, Token> {
        alt((kw_true, kw_false))(input)
    }

    fn decimal(input: &str) -> IResult<&str, i64> {
        map_res(recognize(
            many1(
                terminated(one_of("0123456789"), many0(char('_')))
            )
        ), |out: &str| Result::<i64, &str>::Ok(out.replace("_", "").parse::<i64>().unwrap()))(input)
    }

    fn hexadecimal(input: &str) -> IResult<&str, i64> {
        map_res(
            preceded(
                alt((tag("0x"), tag("0X"))),
                recognize(many1(
                    terminated(one_of("0123456789abcdefABCDEF"), many0(char('_')))
                ))
            ),
            |out: &str| Result::<i64, &str>::Ok(i64::from_str_radix(&str::replace(&out, "_", ""), 16).unwrap())
        )(input)
    }

    fn float(input: &str) -> IResult<&str, Token> {
        map_res(alt((
            // 42.42
            recognize(
                tuple((decimal, char('.'), decimal))
            ),
            // 42e42 and 42.42e42
            recognize(
                tuple((
                    decimal,
                    opt(preceded(
                        char('.'),
                        decimal,
                    )),
                    one_of("eE"),
                    opt(one_of("+-")),
                    decimal
                ))
            )
        )), |out| TResult::Ok(Token::Literal(LiteralKind::Float(out.parse::<f64>().unwrap()))))(input)
    }

    fn integer_literal(input: &str) -> IResult<&str, Token> {
        map_res(alt((decimal, hexadecimal)), |i| TResult::Ok(Token::Literal(LiteralKind::Integer(i))))(input)
    }

    fn string_literal(input: &str) -> IResult<&str, Token> {
        match delimited(char('"'), recognize(many0(none_of("\""))), char('"'))(input) {
            Ok((out, matched)) => Ok((out, Token::Literal(LiteralKind::String(matched.to_string())))),
            Err(e) => Err(e)
        }
    }

    fn literal(input: &str) -> IResult<&str, Token> {
        alt((string_literal, float, integer_literal, boolean))(input)
    }

    many0(alt((literal, identifier, keyword, symbol, whitespace)))(input)
}

// parsing:
// op = { plus | minus | star | slash | pipe | caret }
// ident = @{ (alpha | underscore) ~ (alpha | digit | underscore)* }
// func_params = { "(" ~ (expr ~ (comma ~ expr)*)? ~ ")" }
// func_call = { ident ~ func_params }
// terms_parens = _{ "(" ~ term ~ (op ~ term)* ~ ")" }
// term_bare = _{ func_call | literal | ident  }
// term = { term_bare | terms_parens }
// expr = { term ~ (op ~ term)* }
// assign_stmt = { kw_let ~ ident ~ equal ~ expr  }
// line = { assign_stmt | expr }
// root = _{ SOI ~ WHITESPACE* ~ (line ~ ";")* ~ line? ~ WHITESPACE* ~ EOI }
