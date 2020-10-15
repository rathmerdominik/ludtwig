use super::IResult;
use crate::ast::*;
use crate::parser::general::{document_node, dynamic_context};
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1};
use nom::character::complete::{multispace0, none_of};
use nom::combinator::{cut, opt, value};
use nom::error::context;
use nom::multi::many0;
use nom::sequence::{delimited, preceded, terminated};
use std::collections::HashMap;

static NON_CLOSING_TAGS: [&str; 6] = ["!DOCTYPE", "meta", "input", "img", "br", "hr"];

pub(crate) fn html_tag_argument<'a>(input: &'a str) -> IResult<(&'a str, &'a str)> {
    let (input, _) = multispace0(input)?;
    let (input, key) = take_till1(|c| {
        c == '='
            || c == ' '
            || c == '>'
            || c == '/'
            || c == '<'
            || c == '\n'
            || c == '\r'
            || c == '\t'
    })(input)?; //alphanumeric1(input)?;
    let (input, equal) = opt(tag("=\""))(input)?;

    if equal == None {
        return Ok((input, (key, "")));
    }

    let (input, value) = take_till1(|c| c == '"')(input)?;
    let (input, _) = tag("\"")(input)?;
    let (input, _) = multispace0(input)?;

    Ok((input, (key, value)))
}

pub(crate) fn html_tag_argument_map<'a>(input: &'a str) -> IResult<HashMap<&'a str, &'a str>> {
    let (input, list) = many0(html_tag_argument)(input)?;
    let map = list.into_iter().collect::<HashMap<&str, &str>>();
    Ok((input, map))
}

// returns (tag, self_closed)
pub(crate) fn html_open_tag(input: &str) -> IResult<(&str, bool, HashMap<&str, &str>)> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("<")(input)?;
    let (input, open) = take_till1(|c| {
        c == ' ' || c == '>' || c == '/' || c == '<' || c == '\n' || c == '\r' || c == '\t'
    })(input)?;
    //let (input, _args) = many0(none_of("></"))(input)?;
    let (input, args) = html_tag_argument_map(input)?;

    let (input, mut closed) = alt((value(false, tag(">")), value(true, tag("/>"))))(input)?;

    if NON_CLOSING_TAGS.contains(&open) {
        closed = true;
    }

    Ok((input, (open, closed, args)))
}

pub(crate) fn html_close_tag<'a>(open_tag: &'a str) -> impl Fn(&'a str) -> IResult<&'a str> {
    delimited(
        multispace0,
        delimited(
            tag("</"),
            terminated(cut(tag(open_tag)), many0(none_of(">"))),
            tag(">"),
        ),
        multispace0,
    )
}

pub(crate) fn html_plain_text(input: &str) -> IResult<HtmlNode> {
    let (remaining, plain) = delimited(
        multispace0,
        take_till1(|c| c == '<' || c == '{' || c == '\t' || c == '\r' || c == '\n'),
        multispace0,
    )(input)?;

    Ok((remaining, HtmlNode::Plain(HtmlPlain { plain })))
}

pub(crate) fn html_complete_tag(input: &str) -> IResult<HtmlNode> {
    // TODO: also parser whitespace because it matters in rendering!: https://prettier.io/blog/2018/11/07/1.15.0.html
    let (mut remaining, (open, self_closed, args)) =
        context("open tag expected", html_open_tag)(input)?;
    let mut children = vec![];

    if !self_closed {
        let (remaining_new, children_new) = many0(document_node)(remaining)?;
        let (remaining_new, _close) = preceded(
            multispace0, /*take_till(|c| c == '<')*/
            dynamic_context(
                format!(
                    "Missing closing tag for opening tag '{}' with arguments {:?}",
                    open, args
                ),
                cut(html_close_tag(open)),
            ),
        )(remaining_new)?;
        remaining = remaining_new;
        children = children_new;
    }

    let tag = HtmlTag {
        name: open,
        self_closed,
        arguments: args,
        children,
    };

    Ok((remaining, HtmlNode::Tag(tag)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_tag_positive() {
        assert_eq!(
            html_open_tag("<a href=\"#\">"),
            Ok(("", ("a", false, vec![("href", "#")].into_iter().collect())))
        );
        assert_eq!(html_open_tag("<p>"), Ok(("", ("p", false, HashMap::new()))));
        assert_eq!(
            html_open_tag("<h1>"),
            Ok(("", ("h1", false, HashMap::new())))
        );
        assert_eq!(
            html_open_tag("<h1>"),
            Ok(("", ("h1", false, HashMap::new())))
        );
        assert_eq!(
            html_open_tag("<!DOCTYPE html>"),
            Ok((
                "",
                ("!DOCTYPE", true, vec![("html", "")].into_iter().collect())
            ))
        );
    }

    #[test]
    fn test_open_tag_negative() {
        // TODO: reimplement with error checking.
        /*
        assert_eq!(
            html_open_tag("<a href=\"#\" <p></p>"),
            Err(nom::Err(TwigParseError::Unparseable))
        );
        assert_eq!(
            html_open_tag("</p>"),
            Err(nom::Err(TwigParseError::Unparseable))
        );

         */
    }

    #[test]
    fn test_open_self_closing_tag() {
        assert_eq!(
            html_open_tag("<br/>"),
            Ok(("", ("br", true, HashMap::new())))
        );
        assert_eq!(
            html_open_tag("<a href=\"#\"/>"),
            Ok(("", ("a", true, vec![("href", "#")].into_iter().collect())))
        )
    }

    #[test]
    fn test_open_non_closing_tag() {
        assert_eq!(
            html_open_tag("<meta charset=\"UTF-8\"><title>SomeTitle</title>"),
            Ok((
                "<title>SomeTitle</title>",
                (
                    "meta",
                    true,
                    vec![("charset", "UTF-8")].into_iter().collect()
                )
            ))
        );
    }

    #[test]
    fn test_complete_tag() {
        assert_eq!(
            html_complete_tag("<meta charset=\"UTF-8\"><title>SomeTitle</title>"),
            Ok((
                "<title>SomeTitle</title>",
                HtmlNode::Tag(HtmlTag {
                    name: "meta",
                    self_closed: true,
                    arguments: vec![("charset", "UTF-8")].into_iter().collect(),
                    children: vec![]
                })
            ))
        );

        assert_eq!(
            html_complete_tag("<div><meta charset=\"UTF-8\"><title></title></div>"),
            Ok((
                "",
                HtmlNode::Tag(HtmlTag {
                    name: "div",
                    self_closed: false,
                    arguments: HashMap::new(),
                    children: vec![
                        HtmlNode::Tag(HtmlTag {
                            name: "meta",
                            self_closed: true,
                            arguments: vec![("charset", "UTF-8")].into_iter().collect(),
                            children: vec![]
                        }),
                        HtmlNode::Tag(HtmlTag {
                            name: "title",
                            self_closed: false,
                            arguments: HashMap::new(),
                            children: vec![]
                        })
                    ]
                })
            ))
        );
    }

    #[test]
    fn test_tag_argument() {
        assert_eq!(html_tag_argument("href=\"#\""), Ok(("", ("href", "#"))));
        assert_eq!(
            html_tag_argument("onClick=\"alert('Hello world');\" "),
            Ok(("", ("onClick", "alert('Hello world');")))
        );
        assert_eq!(html_tag_argument("disabled"), Ok(("", ("disabled", ""))));
    }

    #[test]
    fn test_tag_argument_map() {
        let mut map = HashMap::new();
        map.insert("href", "#");
        map.insert("target", "_blank");

        assert_eq!(
            html_tag_argument_map("href=\"#\" \n\t         target=\"_blank\"   "),
            Ok(("", map))
        );
    }
}
