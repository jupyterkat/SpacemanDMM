use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit1, none_of},
    combinator::{map, map_parser, recognize},
    multi::fold_many0,
    sequence::delimited,
};

use super::polyfill::separated_list1_nonoptional;

pub fn quote(input: &str) -> IResult<&str, char> {
    nom::character::complete::char('"')(input)
}

pub fn decimal(input: &str) -> IResult<&str, char> {
    nom::character::complete::char('.')(input)
}

pub fn character(input: &str) -> IResult<&str, char> {
    let (input, c) = none_of("\"")(input)?;
    Ok((input, c))
}

pub fn string(input: &str) -> IResult<&str, String> {
    delimited(
        quote,
        fold_many0(character, String::new, |mut string, c| {
            string.push(c);
            string
        }),
        quote,
    )
    .parse(input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Float(f32),
    Int(u32),
    String(String),
    List(Vec<f32>),
}

pub fn rec_float(input: &str) -> IResult<&str, &str> {
    recognize((digit1, decimal, digit1)).parse(input)
}

pub fn atom_float(input: &str) -> IResult<&str, Value> {
    map(map_parser(rec_float, nom::number::complete::float), |f| {
        Value::Float(f)
    })
    .parse(input)
}

pub fn atom_u32(input: &str) -> IResult<&str, Value> {
    map(nom::character::complete::u32, Value::Int).parse(input)
}

pub fn atom_string(input: &str) -> IResult<&str, Value> {
    map(string, Value::String).parse(input)
}

pub fn atom_list(input: &str) -> IResult<&str, Value> {
    map(
        separated_list1_nonoptional(tag(","), nom::number::complete::float),
        Value::List,
    )
    .parse(input)
}

pub fn atom(input: &str) -> IResult<&str, Value> {
    alt((atom_list, atom_float, atom_u32, atom_string)).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atoms() {
        let float = r#"4.0"#;
        let int = r#"32"#;
        let string = r#""duplicate""#;
        let list = r#"1,2,5.4"#;
        assert_eq!(atom(float), Ok(("", Value::Float(4.0))));
        assert_eq!(atom(int), Ok(("", Value::Int(32))));
        assert_eq!(
            atom(string),
            Ok(("", Value::String("duplicate".to_string())))
        );
        assert_eq!(
            atom(list),
            Ok(("", Value::List(Vec::from([1.0, 2.0, 5.4]))))
        );
    }

    #[test]
    fn test_empty_str() {
        let empty_str = r#""""#;
        assert_eq!(atom(empty_str), Ok(("", Value::String("".to_string()))));
    }
}
