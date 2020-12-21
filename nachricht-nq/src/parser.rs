use nom::{
    character::complete::digit1,
    Finish,
    IResult,
    combinator::{all_consuming, map, map_res, opt, recognize, value},
    sequence::{terminated, tuple, delimited},
    branch::alt,
    bytes::complete::{tag, take_while, escaped_transform, is_not},
    multi::separated_list0,
};
use nachricht::*;
use anyhow::{anyhow, Result};
use base64::decode;
use std::borrow::Cow;

pub fn parse(i: &str) -> Result<Field> {
    Ok(all_consuming(terminated(field, white))(i).finish().map_err(|e| anyhow!("{}", e))?.1)
}

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

fn identifier(i: &str) -> IResult<&str, &str> {
    is_not(" \\$,=\"'()#\n")(i)
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
    map_res(delimited(
        tag("'"),
        b64,
        tag("'")), |c| { decode(c) }
    )(i)
}

fn escaped_string(i: &str) -> IResult<&str, String> {
    delimited(
        tag("\""),
        alt((
            escaped_transform(
                is_not("\\\""),
                '\\',
                alt((
                    value("\\", tag("\\")),
                    value("\n", tag("n")),
                    value("\"", tag("\"")),
                ))
            ),
            map(tag(""), String::from)
        )),
        tag("\""),
    )(i)
}

fn symbol(i: &str) -> IResult<&str, String> {
    alt((
            map(tuple((tag("#"), identifier)), |(_,i)| String::from(i)),
            map(tuple((tag("#"), escaped_string)), |(_,i)| i)
    ))(i)
}

fn container(i: &str) -> IResult<&str, Vec<Field>> {
    delimited(
        tag("("),
        map(tuple((separated_list0(tag(","), field), white, opt(tag(",")), white)), |(l,_,_,_)| l),
        tag(")"),
    )(i)
}

fn nch_value(i: &str) -> IResult<&str, Value> {
    map(tuple((
            white,
            alt((
                map(container, |f| Value::Container(f)),
                map(symbol, |s| Value::Symbol(Cow::Owned(s))),
                map(escaped_string, |s| Value::Str(Cow::Owned(s))),
                map(bytes, |b| Value::Bytes(Cow::Owned(b))),
                map(intn, |i| Value::Int(Sign::Neg, i)),
                map(intp, |i| Value::Int(Sign::Pos, i)),
                map(float32, |f| Value::F32(f)),
                map(float64, |f| Value::F64(f)),
                map(keyword, |k| match k {
                    Keyword::Null => Value::Null,
                    Keyword::True => Value::Bool(true),
                    Keyword::False => Value::Bool(false)
                })
            )),
            white
        )), |(_,v,_)| v)(i)
}

fn key(i: &str) -> IResult<&str, String> {
    alt((
            map(tuple((identifier, white, tag("="))), |(i,_,_)| String::from(i)),
            map(tuple((escaped_string, white, tag("="))), |(i,_,_)| i)
    ))(i)
}

fn field(i: &str) -> IResult<&str, Field> {
    alt((
        map(tuple((white, key, white, nch_value, white)), |(_,k,_,v,_)| Field { name: Some(Cow::Owned(k)), value: v }),
        map(nch_value, |v| Field { name: None, value: v }),
    ))(i)
}

#[cfg(test)]
mod tests {

    use ::nachricht::*;
    use std::borrow::Cow;

    #[test]
    fn primitives() {
        assert_eq!(super::parse("null").unwrap(), Field { name: None, value: Value::Null });
        assert_eq!(super::parse("true").unwrap(), Field { name: None, value: Value::Bool(true) });
        assert_eq!(super::parse("false").unwrap(), Field { name: None, value: Value::Bool(false) });
    }

    #[test]
    fn integers() {
        assert_eq!(super::parse("123").unwrap(), Field { name: None, value: Value::Int(Sign::Pos, 123) });
        assert_eq!(super::parse("-123").unwrap(), Field { name: None, value: Value::Int(Sign::Neg, 123) });
    }

    #[test]
    fn floats() {
        assert_eq!(super::parse("$123").unwrap(), Field { name: None, value: Value::F32(123f32) });
        assert_eq!(super::parse("$$123").unwrap(), Field { name: None, value: Value::F64(123f64) });
    }

    #[test]
    fn strings() {
        assert_eq!(super::parse("\"\"").unwrap(), Field { name: None, value: Value::Str(Cow::Borrowed("")) });
        assert_eq!(super::parse("\"abc\"").unwrap(), Field { name: None, value: Value::Str(Cow::Borrowed("abc")) });
        assert_eq!(super::parse("\"abc\\\"def\"").unwrap(), Field { name: None, value: Value::Str(Cow::Borrowed("abc\"def")) });
        assert_eq!(super::parse("\"abc\\\\def\"").unwrap(), Field { name: None, value: Value::Str(Cow::Borrowed("abc\\def")) });
    }

    #[test]
    fn binary() {
        assert_eq!(super::parse("'base64=='").unwrap(), Field { name: None, value: Value::Bytes(Cow::Borrowed(&[1, 2, 3])) });
    }

    #[test]
    fn symbol() {
        assert_eq!(super::parse("#abc").unwrap(), Field { name: None, value: Value::Symbol(Cow::Borrowed("abc")) });
        assert_eq!(super::parse("#\"a\\\"bc\"").unwrap(), Field { name: None, value: Value::Symbol(Cow::Borrowed("a\"bc")) });
    }

    #[test]
    fn key() {
        assert_eq!(super::parse("true = false").unwrap(), Field { name: Some(Cow::Borrowed("true")), value: Value::Bool(false) });
        assert_eq!(super::parse("\"true\"= #false").unwrap(), Field { name: Some(Cow::Borrowed("true")), value: Value::Symbol(Cow::Borrowed("false")) });
        assert_eq!(super::parse("true =#\"false\"").unwrap(), Field { name: Some(Cow::Borrowed("true")), value: Value::Symbol(Cow::Borrowed("false")) });
    }

    #[test]
    fn container() {
        assert_eq!(super::parse("()").unwrap(), Field { name: None, value: Value::Container(vec![]) });
        assert_eq!(super::parse("x = ()").unwrap(), Field { name: Some(Cow::Borrowed("x")), value: Value::Container(vec![]) });
        assert_eq!(super::parse("(true, x = false)").unwrap(), Field { name: None, value: Value::Container(vec![
                Field { name: None, value: Value::Bool(true) },
                Field { name: Some(Cow::Borrowed("x")), value: Value::Bool(false) },
        ]) });
    }

}
