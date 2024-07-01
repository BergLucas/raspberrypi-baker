#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    ENV(Vec<(String, String)>),
    RUN(String),
    COPY(String, String),
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
pub struct File {
    pub from: FromClause,
    pub instructions: Vec<Instruction>,
}

impl Eq for Instruction {}
impl Eq for FromClause {}
impl Eq for File {}

use nom::{
    bytes::complete::{tag, take_while},
    combinator::opt,
    error::ParseError,
    multi::many0,
    sequence::{separated_pair, tuple},
    Err, IResult,
};

use glob::glob;

///
/// Utility functions
///
fn comsume_ws<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while(|c| c == ' ')(i)
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

fn till_eol<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let (tail, (line, _)) = tuple((take_while(|c| c != '\n'), opt(tag("\n"))))(i)?;
    Ok((tail, line))
}

///
/// Parsers
///

pub(crate) fn parse_cmd<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Instruction, E> {
    let (tail, cmd) = kw_with_ws(i, "CMD")?;
    Ok((tail, Instruction::CMD(cmd.to_string())))
}

pub(crate) fn parse_user<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Instruction, E> {
    let (tail, user) = kw_with_ws(i, "USER")?;
    Ok((tail, Instruction::USER(user.to_string())))
}

pub(crate) fn parse_workdir<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Instruction, E> {
    let (tail, workdir) = kw_with_ws(i, "WORKDIR")?;
    Ok((tail, Instruction::WORKDIR(workdir.to_string())))
}

pub(crate) fn parse_copy<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Instruction, E> {
    let (paths, _) = tuple((nom::bytes::complete::tag("COPY"), comsume_ws))(i)?;
    let (tail, (src, dest)) = separated_pair(non_space, tag(" "), till_eol)(paths)?;
    if !is_glob_pattern(src) || !is_glob_pattern(dest) {
        return Err(nom::Err::Error(E::from_error_kind(
            i,
            nom::error::ErrorKind::Verify,
        )));
    }

    Ok((tail, Instruction::COPY(src.to_string(), dest.to_string())))
}

pub(crate) fn parse_from<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, FromClause, E> {
    //TODO: parse tag and platform
    let (tail, (_, _, image)) = tuple((tag("FROM"), comsume_ws, till_eol))(i)?;
    Ok((
        tail,
        FromClause {
            image: image.to_string(),
            tag: None,
            platform: None,
        },
    ))
}

pub(crate) fn parse_instructions<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Vec<Instruction>, E> {
    many0(nom::branch::alt((
        parse_cmd,
        parse_user,
        parse_workdir,
        parse_copy,
        //parse_run,
        //parse_env,
    )))(i)
}

pub(crate) fn parse_baker_file<'a, E: ParseError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, File, E> {
    let (insts, from) = parse_from(i)?;
    let (tail, instructions) = parse_instructions(insts)?;
    Ok((tail, File { from, instructions }))
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
    let input = "FROM ubuntu\nUSER root\nCMD echo hello";
    let (_, res) = parse_baker_file::<()>(input).unwrap();
    assert_eq!(
        res,
        File {
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
