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

pub fn parse(i: &str) -> Result<Value> {
    Ok(all_consuming(terminated(nch_value, white))(i).finish().map_err(|e| anyhow!("{}", e))?.1)
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
    is_not(" \\$,:\"'()#\n")(i)
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

fn array(i: &str) -> IResult<&str, Vec<Value>> {
    delimited(
        tag("["),
        map(tuple((separated_list0(tag(","), nch_value), white, opt(tag(",")), white)), |(l,_,_,_)| l),
        tag("]"),
    )(i)
}

fn nch_map(i: &str) -> IResult<&str, Vec<(Value, Value)>> {
    delimited(
        tag("{"),
        map(tuple((separated_list0(tag(","), entry), white, opt(tag(",")), white)), |(l,_,_,_)| l),
        tag("}"),
    )(i)
}

fn record(i: &str) -> IResult<&str, Vec<(String, Value)>> {
    delimited(
        tag("("),
        map(tuple((separated_list0(tag(","), field), white, opt(tag(",")), white)), |(l,_,_,_)| l),
        tag(")"),
    )(i)
}

fn entry(i: &str) -> IResult<&str, (Value, Value)> {
    map(tuple((nch_value, white, tag(":"), white, nch_value)), |(l,_,_,_,r)| (l, r))(i)
}

fn field(i: &str) -> IResult<&str, (String, Value)> {
    map(tuple((white, key, white, tag(":"), white, nch_value)), |(_,l,_,_,_,r)| (l, r))(i)
}

fn nch_value(i: &str) -> IResult<&str, Value> {
    map(tuple((
            white,
            alt((
                map(array, |f| Value::Array(f)),
                map(nch_map, |f| Value::Map(f)),
                map(record, |f| Value::Record(f.into_iter().map(|(k, v)| (Cow::Owned(k), v)).collect())),
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
            map(identifier, |i| String::from(i)),
            escaped_string,
    ))(i)
}

#[cfg(test)]
mod tests {

    use ::nachricht::*;
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    #[test]
    fn primitives() {
        assert_eq!(super::parse("null").unwrap(), Value::Null);
        assert_eq!(super::parse("true").unwrap(), Value::Bool(true));
        assert_eq!(super::parse("false").unwrap(), Value::Bool(false));
    }

    #[test]
    fn integers() {
        assert_eq!(super::parse("123").unwrap(), Value::Int(Sign::Pos, 123));
        assert_eq!(super::parse("-123").unwrap(), Value::Int(Sign::Neg, 123));
    }

    #[test]
    fn floats() {
        assert_eq!(super::parse("$123").unwrap(), Value::F32(123f32));
        assert_eq!(super::parse("$$123").unwrap(), Value::F64(123f64));
    }

    #[test]
    fn strings() {
        assert_eq!(super::parse("\"\"").unwrap(), Value::Str(Cow::Borrowed("")));
        assert_eq!(super::parse("\"abc\"").unwrap(), Value::Str(Cow::Borrowed("abc")));
        assert_eq!(super::parse("\"abc\\\"def\"").unwrap(), Value::Str(Cow::Borrowed("abc\"def")));
        assert_eq!(super::parse("\"abc\\\\def\"").unwrap(), Value::Str(Cow::Borrowed("abc\\def")));
    }

    #[test]
    fn binary() {
        assert_eq!(super::parse("'base64//'").unwrap(), Value::Bytes(Cow::Borrowed(&[109, 171, 30, 235, 143, 255])));
    }

    #[test]
    fn symbol() {
        assert_eq!(super::parse("#abc").unwrap(), Value::Symbol(Cow::Borrowed("abc")));
        assert_eq!(super::parse("#\"a\\\"bc\"").unwrap(), Value::Symbol(Cow::Borrowed("a\"bc")));
    }

    #[test]
    fn array() {
        assert_eq!(super::parse("[]").unwrap(), Value::Array(Vec::new()));
        assert_eq!(super::parse("[true, false]").unwrap(), Value::Array(vec![
                    Value::Bool(true),
                    Value::Bool(false),
        ]));
    }

    #[test]
    fn record() {
        assert_eq!(super::parse("()").unwrap(), Value::Record(BTreeMap::new()));
        assert_eq!(super::parse("(x: true, y: false)").unwrap(), Value::Record(BTreeMap::from([
                    (Cow::Borrowed("x"), Value::Bool(true)),
                    (Cow::Borrowed("y"), Value::Bool(false)),
        ])));
    }

    #[test]
    fn map() {
        assert_eq!(super::parse("{}").unwrap(), Value::Map(Vec::new()));
        assert_eq!(super::parse("{\"x\": true, \"y\": false}").unwrap(), Value::Map(vec![
                    (Value::Str(Cow::Borrowed("x")), Value::Bool(true)),
                    (Value::Str(Cow::Borrowed("y")), Value::Bool(false)),
        ]));
    }

    #[test]
    fn canonical() {
        let message = "( cats: [ ( name: \"Jessica\", species: #PrionailurusViverrinus, ), ( name: \"Wantan\", species: #LynxLynx, ), ( name: \"Sphinx\", species: #FelisCatus, ), ( name: \"Chandra\", species: #PrionailurusViverrinus, ), ], version: 1, )";
        let expected = Value::Record(BTreeMap::from([
            (Cow::Borrowed("cats"), Value::Array(vec![
                Value::Record(BTreeMap::from([
                    (Cow::Borrowed("name"), Value::Str(Cow::Borrowed("Jessica"))),
                    (Cow::Borrowed("species"), Value::Symbol(Cow::Borrowed("PrionailurusViverrinus"))),
                ])),
                Value::Record(BTreeMap::from([
                    (Cow::Borrowed("name"), Value::Str(Cow::Borrowed("Wantan"))),
                    (Cow::Borrowed("species"), Value::Symbol(Cow::Borrowed("LynxLynx"))),
                ])),
                Value::Record(BTreeMap::from([
                    (Cow::Borrowed("name"), Value::Str(Cow::Borrowed("Sphinx"))),
                    (Cow::Borrowed("species"), Value::Symbol(Cow::Borrowed("FelisCatus"))),
                ])),
                Value::Record(BTreeMap::from([
                    (Cow::Borrowed("name"), Value::Str(Cow::Borrowed("Chandra"))),
                    (Cow::Borrowed("species"), Value::Symbol(Cow::Borrowed("PrionailurusViverrinus"))),
                ])),
            ])),
            (Cow::Borrowed("version"), Value::Int(Sign::Pos, 1)),
        ]));
        assert_eq!(super::parse(&message).unwrap(), expected);
    }

}
