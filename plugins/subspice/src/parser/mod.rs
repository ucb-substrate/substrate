use std::str;

use nom::branch::alt;
use nom::bytes::complete::{tag_no_case, take_till, take_till1};
use nom::character::complete::{line_ending, multispace0, space0, space1};
use nom::character::streaming::char;
use nom::combinator::opt;
use nom::multi::{many0, many1};
use nom::sequence::{delimited, pair, preceded, tuple};
use nom::IResult;
use serde::Serialize;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum SpiceLine<'a> {
    Subckt(SubcktLine<'a>),
    Comment(&'a str),
    Other,
}

impl<'a> SpiceLine<'a> {
    pub fn subckt(&self) -> Option<&SubcktLine> {
        match self {
            SpiceLine::Subckt(line) => Some(line),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct SubcktLine<'a> {
    pub name: &'a str,
    pub ports: Vec<&'a str>,
}

fn is_newline(c: char) -> bool {
    c == '\n' || c == '\r'
}

fn is_space_or_line(c: char) -> bool {
    c == '\n' || c == '\r' || c == ' ' || c == '\t'
}

fn within_line_space1(input: &str) -> IResult<&str, ()> {
    let (input, _) = space1(input)?;
    Ok((input, ()))
}

fn line_continuation1(input: &str) -> IResult<&str, ()> {
    let (input, _) = tuple((
        space0,
        opt(line_comment),
        line_ending,
        space0,
        char('+'),
        space0,
    ))(input)?;
    Ok((input, ()))
}

fn many_line_continuation1(input: &str) -> IResult<&str, ()> {
    let (input, _) = many1(line_continuation1)(input)?;
    Ok((input, ()))
}

fn spice_space1(input: &str) -> IResult<&str, ()> {
    let (input, _) = alt((many_line_continuation1, within_line_space1))(input)?;
    Ok((input, ()))
}

fn line_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = tuple((space0, char(';'), take_till(is_newline)))(input)?;
    Ok((input, ()))
}

fn ident(input: &str) -> IResult<&str, &str> {
    take_till1(is_space_or_line)(input)
}

fn subckt_ports(input: &str) -> IResult<&str, Vec<&str>> {
    many0(preceded(spice_space1, ident))(input)
}

fn subckt_name(input: &str) -> IResult<&str, &str> {
    preceded(spice_space1, ident)(input)
}

fn subckt_line(input: &str) -> IResult<&str, SpiceLine> {
    let (input, (_, name, ports)) =
        tuple((tag_no_case(".subckt"), subckt_name, subckt_ports))(input)?;

    Ok((input, SpiceLine::Subckt(SubcktLine { name, ports })))
}

fn comment_line(input: &str) -> IResult<&str, SpiceLine> {
    let (input, (_, _, comment, _)) =
        tuple((space0, tag_no_case("*"), take_till(is_newline), line_ending))(input)?;
    Ok((input, SpiceLine::Comment(comment.trim())))
}

fn other_line(input: &str) -> IResult<&str, SpiceLine> {
    let (input, _) = pair(ident, many0(preceded(ident, spice_space1)))(input)?;
    Ok((input, SpiceLine::Other))
}

fn spice_line(input: &str) -> IResult<&str, SpiceLine> {
    alt((subckt_line, comment_line, other_line))(input)
}

pub(crate) fn parse_spice(input: &str) -> IResult<&str, Vec<SpiceLine>> {
    many0(delimited(multispace0, spice_line, multispace0))(input)
}
