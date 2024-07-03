use std::path::PathBuf;

use glob::glob;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_while},
    character::complete::{space0, space1},
    combinator::opt,
    error::ParseError,
    multi::many0,
    sequence::{preceded, separated_pair, tuple},
    Err, IResult,
};
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    ENV(Vec<(String, String)>),
    RUN(String),
    COPY(String, PathBuf),
    WORKDIR(String),
    USER(String),
    CMD(String),
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct FromClause {
    pub image: String,
    pub tag: Option<String>,
    pub platform: Option<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct BakerFile {
    pub from: FromClause,
    pub instructions: Vec<Instruction>,
}

impl Eq for Instruction {}
impl Eq for FromClause {}
impl Eq for BakerFile {}

///
/// Utility functions
///
fn comsume_ws<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    space0(input)
}

fn kw_with_ws<'a, E: ParseError<&'a str>>(i: &'a str, kw: &'a str) -> IResult<&'a str, &'a str, E> {
    let (tail, (_, _, line)) = tuple((tag(kw), comsume_ws, till_eol))(i)?;
    Ok((tail, line))
}

fn is_glob_pattern(path: &str) -> bool {
    glob(path).is_ok()
}

fn non_space<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while(|c| c != ' ')(i)
}

fn eol(ch: char) -> bool {
    ch == '\n' || ch == '\r'
}

fn till_eol<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let (tail, (line, _)) = tuple((take_till(eol), opt(alt((tag("\r\n"), tag("\n"))))))(i)?;
    Ok((tail, line))
}

fn consume_eol<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let (tail, _) = alt((tag("\r\n"), tag("\n")))(i)?;
    Ok((tail, ""))
}
fn consume_blank_line<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let (tail, _) = tuple((comsume_ws, opt(consume_eol)))(i)?;
    Ok((tail, ""))
}

///
/// Parsers
///

fn parse_cmd<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (tail, cmd) = kw_with_ws(i, "CMD")?;
    Ok((tail, Instruction::CMD(cmd.to_string())))
}

fn parse_user<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (tail, user) = kw_with_ws(i, "USER")?;
    Ok((tail, Instruction::USER(user.to_string())))
}

fn parse_workdir<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (tail, workdir) = kw_with_ws(i, "WORKDIR")?;
    Ok((tail, Instruction::WORKDIR(workdir.to_string())))
}

fn parse_copy<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (paths, _) = tuple((nom::bytes::complete::tag("COPY"), comsume_ws))(i)?;
    let (tail, (src, dest)) = separated_pair(non_space, tag(" "), till_eol)(paths)?;
    if !is_glob_pattern(src) || !is_glob_pattern(&dest) {
        return Err(Err::Failure(E::from_error_kind(
            i,
            nom::error::ErrorKind::Fail,
        )));
    }

    Ok((tail, Instruction::COPY(src.to_string(), dest.into())))
}

fn parse_run<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (tail, run) = kw_with_ws(i, "RUN")?;
    Ok((tail, Instruction::RUN(run.to_string())))
}

fn parse_tag<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let (tail, (_, tag)) = tuple((tag(":"), till_eol))(i)?;
    Ok((tail, tag))
}

fn parse_from<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, FromClause, E> {
    let (tail, (_, _, line)) = tuple((tag("FROM"), comsume_ws, till_eol))(i)?;
    let (img, pt) = opt(tuple((
        tag("--platform"),
        comsume_ws,
        non_space,
        comsume_ws,
    )))(line)?;
    let platform = pt.map(|(_, _, p, _)| p.to_string());
    let (last, image) = take_till(|ch| eol(ch) || ch == ':')(img)?;
    let tag = if !last.is_empty() {
        let (_, tag) = parse_tag(last)?;
        Some(tag.to_string())
    } else {
        None
    };

    Ok((
        tail,
        FromClause {
            image: image.to_string(),
            tag,
            platform,
        },
    ))
}

fn parse_env<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (tail, (_, _, envs)) = tuple((tag("ENV"), comsume_ws, till_eol))(i)?;
    let envs = envs
        .split_whitespace()
        .map(|env| {
            let mut env = env.split('=');
            let key = env.next().unwrap().to_string();
            let value = env.next().unwrap().to_string();
            (key, value)
        })
        .collect();
    Ok((tail, Instruction::ENV(envs)))
}

fn parse_instruction<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (tail, (_, instruction)) = tuple((
        consume_blank_line,
        nom::branch::alt((
            parse_cmd,
            parse_user,
            parse_workdir,
            parse_copy,
            parse_run,
            parse_env,
        )),
    ))(i)?;
    Ok((tail, instruction))
}

fn parse_instructions<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Vec<Instruction>, E> {
    many0(parse_instruction)(i)
}

pub fn parse_baker_file<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, BakerFile, E> {
    let (insts, from) = parse_from(i)?;
    let (tail, instructions) = parse_instructions(insts)?;
    Ok((tail, BakerFile { from, instructions }))
}

#[test]
fn test_parse_copy() {
    let input = "COPY /src/* /dest\n";
    let (_, res) = parse_copy::<()>(input).unwrap();
    assert_eq!(res, Instruction::COPY("/src/*".to_string(), "/dest".into()));
}

#[test]
fn test_parse_workdir() {
    let input = "WORKDIR /src\n";
    let (_, res) = parse_workdir::<()>(input).unwrap();
    assert_eq!(res, Instruction::WORKDIR("/src".to_string()));
}

#[test]
fn test_parse_user() {
    let input = "USER root\n";
    let (_, res) = parse_user::<()>(input).unwrap();
    assert_eq!(res, Instruction::USER("root".to_string()));
}

#[test]
fn test_parse_cmd() {
    let input = "CMD echo hello\n";
    let (_, res) = parse_cmd::<()>(input).unwrap();
    assert_eq!(res, Instruction::CMD("echo hello".to_string()));
}

#[test]
fn test_parse_run() {
    let input = "RUN echo hello\n";
    let (_, res) = parse_run::<()>(input).unwrap();
    assert_eq!(res, Instruction::RUN("echo hello".to_string()));
}

#[test]
fn test_parse_env() {
    let input = "ENV KEY1=VALUE1 KEY2=VALUE2";
    let (_, res) = parse_env::<()>(input).unwrap();
    assert_eq!(
        res,
        Instruction::ENV(vec![
            ("KEY1".to_string(), "VALUE1".to_string()),
            ("KEY2".to_string(), "VALUE2".to_string())
        ])
    );
}

#[test]
fn test_parse_from_full_options() {
    let input = "FROM --platform x86 ubuntu:latest\n";
    let (_, res) = parse_from::<()>(input).unwrap();
    assert_eq!(
        res,
        FromClause {
            image: "ubuntu".to_string(),
            tag: Some("latest".to_string()),
            platform: Some("x86".to_string())
        }
    );
}

#[test]
fn test_parse_from_no_options() {
    let input = "FROM ubuntu\n";
    let (_, res) = parse_from::<()>(input).unwrap();
    assert_eq!(
        res,
        FromClause {
            image: "ubuntu".to_string(),
            tag: None,
            platform: None
        }
    );
}

#[test]
fn test_parse_baker_file() {
    let input = "FROM ubuntu\nUSER root\n   \nCMD echo hello";
    let (_, res) = parse_baker_file::<()>(input).unwrap();
    assert_eq!(
        res,
        BakerFile {
            from: FromClause {
                image: "ubuntu".to_string(),
                tag: None,
                platform: None
            },
            instructions: vec![
                Instruction::USER("root".to_string()),
                Instruction::CMD("echo hello".to_string()),
            ]
        }
    );
}

#[test]
fn test_parse_instructions() {
    let input = "USER root\nCMD echo hello";
    let (_, res) = parse_instructions::<()>(input).unwrap();
    assert_eq!(
        res,
        vec![
            Instruction::USER("root".to_string()),
            Instruction::CMD("echo hello".to_string()),
        ]
    );
}
