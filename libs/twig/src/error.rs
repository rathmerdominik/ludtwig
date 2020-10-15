use nom::error::ErrorKind;
use nom::lib::std::fmt::Formatter;
use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub struct ParsingErrorInformation<I> {
    input: I,
    kind: ErrorKind,
    context: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum TwigParseError<I> {
    ParsingError(ParsingErrorInformation<I>),
    ParsingFailure(ParsingErrorInformation<I>),
    MissingClosing,
}

//impl<I> Error for TwigParseError<I> {}

impl<I: std::fmt::Debug> Display for TwigParseError<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TwigParseError::ParsingError(info) => write!(
                f,
                "parsing error because: ({:?}, {:?}, {:?})",
                info.input, info.kind, info.context
            ),
            TwigParseError::ParsingFailure(info) => write!(
                f,
                "Unrecoverable parsing failure because: ({:?}, {:?}, {:?})",
                info.input, info.kind, info.context
            ),
            TwigParseError::MissingClosing => write!(f, "Missing closing tag / block"),
        }
    }
}

impl<I: std::fmt::Debug> nom::error::ParseError<I> for ParsingErrorInformation<I> {
    fn from_error_kind(_input: I, _kind: ErrorKind) -> Self {
        println!("[FROM_ERROR_KIND] {:?}: {:?}", _kind, _input);

        ParsingErrorInformation {
            input: _input,
            kind: _kind,
            context: None,
        }
    }

    fn append(_input: I, _kind: ErrorKind, other: Self) -> Self {
        println!("[APPEND] {:?}: {:?}", _kind, _input);
        other
    }

    fn from_char(input: I, _: char) -> Self {
        println!("[FROM_CHAR] {:?}", input);
        ParsingErrorInformation {
            input,
            kind: ErrorKind::Not,
            context: None,
        }
    }

    fn add_context(_input: I, _ctx: &str, mut other: Self) -> Self {
        println!("[ADD_CONTEXT] {} {:?} {:?}", _ctx, _input, other);
        other.context = Some(_ctx.to_string());

        other
    }
}

pub(crate) trait DynamicParseError<I> {
    fn add_dynamic_context(input: I, ctx: String, other: Self) -> Self;
}

impl<I: std::fmt::Debug> DynamicParseError<I> for ParsingErrorInformation<I> {
    fn add_dynamic_context(_input: I, _ctx: String, mut other: Self) -> Self {
        println!("[ADD_DYNAMIC_CONTEXT] {:?} {:?} {:?}", _ctx, _input, other);
        other.context = Some(_ctx);

        other
    }
}

impl<I> From<nom::Err<ParsingErrorInformation<I>>> for TwigParseError<I> {
    fn from(e: nom::Err<ParsingErrorInformation<I>>) -> Self {
        match e {
            nom::Err::Incomplete(_) => unreachable!(),
            nom::Err::Error(i) => TwigParseError::ParsingError(i),
            nom::Err::Failure(i) => TwigParseError::ParsingFailure(i),
        }
    }
}

// error reporting logic
impl TwigParseError<&str> {
    pub fn pretty_print_userfriendly_error(&self, input: &str) {
        let info = match self {
            TwigParseError::ParsingError(i) => i,
            TwigParseError::ParsingFailure(i) => i,
            TwigParseError::MissingClosing => panic!("unprintable error"),
        };

        let (line, column, last_line) = get_line_and_column_of_subslice(input, info.input);

        println!(
            "Parsing goes wrong in line {} and column {} :",
            line, column
        );

        println!("{}", last_line);

        for _ in 0..(column - 1) {
            print!(" ");
        }

        print!("^\n");

        for _ in 0..(column - 1) {
            print!(" ");
        }

        print!("|\n");

        println!("{:?}", info.kind);

        match &info.context {
            None => println!("{:?}", info.kind),
            Some(c) => println!("{}", c),
        }
    }
}

pub trait SubsliceOffset {
    /**
    Returns the byte offset of an inner slice relative to an enclosing outer slice.

    Examples

    ```ignore
    let string = "a\nb\nc";
    let lines: Vec<&str> = string.lines().collect();
    assert!(string.subslice_offset_stable(lines[0]) == Some(0)); // &"a"
    assert!(string.subslice_offset_stable(lines[1]) == Some(2)); // &"b"
    assert!(string.subslice_offset_stable(lines[2]) == Some(4)); // &"c"
    assert!(string.subslice_offset_stable("other!") == None);
    ```
    */
    fn subslice_offset(&self, inner: &Self) -> Option<usize>;
}

impl SubsliceOffset for str {
    fn subslice_offset(&self, inner: &str) -> Option<usize> {
        let self_beg = self.as_ptr() as usize;
        let inner = inner.as_ptr() as usize;
        if inner < self_beg || inner > self_beg.wrapping_add(self.len()) {
            None
        } else {
            Some(inner.wrapping_sub(self_beg))
        }
    }
}

fn get_line_and_column_of_subslice<'a>(input: &'a str, slice: &'a str) -> (usize, usize, &'a str) {
    let offset = input.subslice_offset(slice).unwrap();
    let mut last_line_start = 0;
    let mut last_line_end = 0;
    let mut found = false;
    let mut lines = 1;
    let mut byte_number = 0;

    for (i, byte) in input.bytes().enumerate() {
        byte_number = i;
        if byte == b'\r' || byte == b'\n' {
            lines += 1;
            last_line_end = i + 1;

            if found {
                break;
            }

            last_line_start = last_line_end;
        }

        if i == offset {
            found = true;
        }
    }

    // if the for loop did not found a newline in the last parsed line the end and start will be the same.
    if last_line_start == last_line_end {
        last_line_end = byte_number + 1;
    } else {
        last_line_end -= 1;
    }

    let last_line = &input[last_line_start..last_line_end];
    let column = offset - last_line_start + 1;

    (lines, column, last_line)

    //todo!();
    /*
    let offset = input.subslice_offset(slice).unwrap();
    let before = &input[..offset];
    let line_count = before.lines().count();
    let last_line = match before.lines().last() {
        None => "",
        Some(l) => l,
    };

    (before.lines().count(), 0, last_line)
     */
}