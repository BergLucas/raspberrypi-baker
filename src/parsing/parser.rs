#[derive(PartialEq, Debug, Clone)]
pub enum Instruction {
    ENV(Vec<(String, String)>),
    RUN(String),
    COPY(String, String),
    WORKDIR(String),
    USER(String),
    CMD(String),
}
use std::path::PathBuf;

#[derive(Debug, PartialEq, Default)]
pub struct FromClause {
    pub image: String,
    pub tag: Option<String>,
    pub platform: Option<String>,
}
#[derive(Debug, PartialEq)]
pub struct File {
    pub from: FromClause,
    pub instructions: Vec<Instruction>,
}

use nom::{
    bytes::complete::{tag, take_till, take_while},
    error::ParseError,
    multi::many0,
    sequence::{separated_pair, tuple},
    Err, IResult, Parser,
};

use glob::glob;

///
/// Utility functions
///
fn comsume_ws<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while(|c| c == ' ')(i)
}

fn kw_with_ws<'a, E: ParseError<&'a str>>(i: &'a str, kw: &'a str) -> IResult<&'a str, &'a str, E> {
    let (_, (_, _, line)) = tuple((tag(kw), comsume_ws, till_eol))(i)?;
    Ok((i, line))
}

fn is_glob_pattern(path: &str) -> bool {
    glob(path).is_ok()
}

fn non_space<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while(|c| c != ' ')(i)
}

fn till_eol<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while(|c| c != '\n')(i)
}

///
/// Parsers
///

fn parse_cmd<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (_, cmd) = kw_with_ws(i, "CMD")?;
    Ok((i, Instruction::CMD(cmd.to_string())))
}

fn parse_user<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (_, user) = kw_with_ws(i, "USER")?;
    Ok((i, Instruction::USER(user.to_string())))
}

fn parse_workdir<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (_, workdir) = kw_with_ws(i, "WORKDIR")?;
    let path = PathBuf::from(workdir);
    if !path.is_dir() {
        panic!("Invalid path")
    }
    Ok((i, Instruction::WORKDIR(path.to_str().unwrap().to_string())))
}

fn parse_copy<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (paths, _) = tuple((nom::bytes::complete::tag("COPY"), comsume_ws))(i)?;
    let (_, (src, dest)) = separated_pair(non_space, tag(" "), till_eol)(paths)?;
    if !is_glob_pattern(src) || !is_glob_pattern(dest) {
        panic!("Invalid path format")
    }

    Ok((i, Instruction::COPY(src.to_string(), dest.to_string())))
}

fn parse_instruction<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Instruction, E> {
    let (i, instruction) = nom::branch::alt((
        parse_cmd,
        parse_user,
        parse_workdir,
        parse_copy,
        //parse_run,
        //parse_env,
    ))(i)?;
    Ok((i, instruction))
}

fn parse_from<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, FromClause, E> {
    //TODO: parse tag and platform
    let (_, (_, _, image)) = tuple((tag("FROM"), comsume_ws, till_eol))(i)?;
    Ok((
        i,
        FromClause {
            image: image.to_string(),
            tag: None,
            platform: None,
        },
    ))
}

fn parse_instructions<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Vec<Instruction>, E> {
    let (_, instructions) = many0(parse_instruction)(i)?;
    Ok((i, instructions))
}

fn parse_baker_file<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, File, E> {
    //let (i, from) = parse_from(i)?;
    let ex: Result<(&str, Vec<Instruction>), Err<_>> = parse_instructions::<()>(i);
    let (i, instructions) = match ex {
        Ok((i, res)) => (i, res),
        Err(e) => panic!("Error: {:?}", e),
    };
    Ok((
        i,
        File {
            from: FromClause::default(),
            instructions,
        },
    ))
}

#[test]
fn test_parse_copy() {
    let input = "COPY /src/* /dest\n";
    let (_, res) = parse_copy::<()>(input).unwrap();
    assert_eq!(
        res,
        Instruction::COPY("/src/*".to_string(), "/dest".to_string())
    );
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
fn test_parse_from() {
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
    let input = "FROM ubuntu\nCMD echo hello\n";
    let (_, res) = parse_baker_file::<()>(input).unwrap();
    assert_eq!(
        res,
        File {
            from: FromClause::default(),
            instructions: vec![Instruction::CMD("echo hello".to_string())]
        }
    );
}

#[test]
fn test_parse_instruction() {
    let input = "CMD echo hello\n";
    let (_, res) = parse_instruction::<()>(input).unwrap();
    assert_eq!(res, Instruction::CMD("echo hello".to_string()));
}

#[test]
fn test_parse_instructions() {
    let input = "CMD echo hello\nUSER root\n";
    let (_, res) = parse_instructions::<()>(input).unwrap();
    assert_eq!(
        res,
        vec![
            Instruction::CMD("echo hello".to_string()),
            Instruction::USER("root".to_string())
        ]
    );
}
