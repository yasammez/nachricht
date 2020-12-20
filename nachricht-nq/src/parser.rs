use nom::{
    character::complete::{digit1, none_of, one_of, alpha1},
    Finish,
    IResult,
    combinator::{all_consuming, map, map_res, opt, recognize, value},
    sequence::{terminated, tuple, delimited},
    branch::alt,
    bytes::complete::{tag, take_while, escaped_transform, is_not, take_till, take_until},
};
use nachricht::*;
use anyhow::{anyhow, Result};
use base64::decode;
use std::borrow::Cow;

pub enum Keyword {
    Null,
    True,
    False,
}

const WHITESPACE: &'static str = " \t\r\n";
const B64_CHARS: &'static str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz01234567890+/";

fn white(i: &str) -> IResult<&str, &str> {
    take_while(move |c| WHITESPACE.contains(c))(i)
}

fn keyword(i: &str) -> IResult<&str, Keyword> {
    alt((
            map(tag("null"), |_| Keyword::Null),
            map(tag("true"), |_| Keyword::True),
            map(tag("false"),|_| Keyword::False)
    ))(i)
}

fn float(i: &str) -> IResult<&str, &str> {
    recognize(tuple((opt(tag("-")), opt(digit1), opt(tag(".")), opt(digit1))))(i)
}

fn float32(i: &str) -> IResult<&str, f32> {
    map_res(tuple((tag("$"), float)), |(_,n)| n.parse())(i)
}

fn float64(i: &str) -> IResult<&str, f64> {
    map_res(tuple((tag("$$"), float)), |(_,n)| n.parse())(i)
}

fn intn(i: &str) -> IResult<&str, u64> {
    map_res(tuple((tag("-"), digit1)), |(_,n): (&str, &str)| n.parse())(i)
}

fn intp(i: &str) -> IResult<&str, u64> {
    map_res(digit1, |n: &str| n.parse())(i)
}

fn b64(i: &str) -> IResult<&str, &str> {
    recognize(tuple((take_while(move |c| B64_CHARS.contains(c)), opt(tag("=")), opt(tag("=")))))(i)
}

fn bytes(i: &str) -> IResult<&str, Vec<u8>> {
    map_res(tuple((tag(":"), b64)), |(_,b)| decode(b))(i)
}

fn escaped(i: &str) -> IResult<&str, String> {
    escaped_transform(
        none_of("\""), 
        '"',
        nom::combinator::value("\"", tag("\"\""))
    )(i)
}

fn string(i: &str) -> IResult<&str, String> {
    delimited(
            tag("\""),
            map(opt(escaped_transform(
                none_of("\\\""),
                '\\',
                alt((
                        tag("\\"),
                        tag("\""),
                )))), |c| c.unwrap_or("".into())),
            tag("\"")
    )(i)
    
}

fn nch_value(i: &str) -> IResult<&str, Value> {
    alt((
        map(string, |s| Value::Str(Cow::Owned(s))),
        map(bytes, |b| Value::Bytes(Cow::Owned(b))),
        map(intn, |i| Value::Int(Sign::Neg, i)),
        map(intp, |i| Value::Int(Sign::Pos, i)),
        map(float32, |f| Value::F32(f)),
        map(float64, |f| Value::F64(f)),
        map(keyword, |k| match k {
            Keyword::Null => Value::Null,
            Keyword::True => Value::Bool(true),
            Keyword::False => Value::Bool(false)
    })))(i)
}

fn field(i: &str) -> IResult<&str, Field> {
    map(nch_value, |v| Field { name: None, value: v })(i)
}

pub fn parse(i: &str) -> Result<Field> {
    Ok(all_consuming(terminated(field, white))(i).finish().map_err(|e| anyhow!("{}", e))?.1)
}
