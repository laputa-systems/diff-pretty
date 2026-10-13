use std::ffi::{OsStr, OsString};
use std::io::BufReader;
use std::path::PathBuf;

use scrl::{PagingMode, RunOptions, SessionOptions};

#[derive(Debug, PartialEq, Eq)]
struct ParsedArgs {
    paging: PagingMode,
    wrap: bool,
    follow: bool,
    filter: Option<String>,
    paths: Vec<PathBuf>,
    help: bool,
}

fn parse_args(arguments: impl IntoIterator<Item = OsString>) -> Result<ParsedArgs, String> {
    let mut parsed = ParsedArgs {
        paging: PagingMode::Auto,
        wrap: false,
        follow: false,
        filter: None,
        paths: Vec::new(),
        help: false,
    };
    for argument in arguments {
        if argument == OsStr::new("--no-pager") {
            parsed.paging = PagingMode::Never;
        } else if argument == OsStr::new("--wrap") {
            parsed.wrap = true;
        } else if argument == OsStr::new("--follow") {
            parsed.follow = true;
        } else if argument == OsStr::new("--help") || argument == OsStr::new("-h") {
            parsed.help = true;
            return Ok(parsed);
        } else if let Some(argument) = argument.to_str() {
            if let Some(value) = argument.strip_prefix("--filter=") {
                parsed.filter = Some(value.to_owned());
            } else if let Some(value) = argument.strip_prefix("--paging=") {
                parsed.paging = match value {
                    "auto" => PagingMode::Auto,
                    "always" => PagingMode::Always,
                    "never" => PagingMode::Never,
                    _ => return Err(format!("invalid paging mode: {value}")),
                };
            } else {
                parsed.paths.push(PathBuf::from(argument));
            }
        } else {
            parsed.paths.push(PathBuf::from(argument));
        }
    }
    Ok(parsed)
}

fn main() {
    let args = match parse_args(std::env::args_os().skip(1)) {
        Ok(arguments) => arguments,
        Err(error) => {
            eprintln!("scrl: {error}");
            std::process::exit(2);
        }
    };
    if args.help {
        println!(
            "usage: scrl [--paging=auto|always|never] [--no-pager] [--wrap] [--follow] [--filter=REGEX] [FILE ...]"
        );
        return;
    }
    let options = RunOptions {
        paging: args.paging,
        session: SessionOptions {
            title: "scrl".into(),
            search_history: Vec::new(),
            wrap: args.wrap,
            follow: args.follow,
            filter: args.filter,
        },
    };
    let result = if args.paths.is_empty() {
        scrl::run_reader(BufReader::new(std::io::stdin()), options)
    } else if args.paths.len() == 1 {
        scrl::run_source(scrl::FileSource::new(&args.paths[0]), options)
    } else {
        scrl::run_source(scrl::FilesSource::new(&args.paths), options)
    };
    if let Err(error) = result {
        eprintln!("scrl: {error}");
        std::process::exit(1);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::{PagingMode, parse_args};
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    #[test]
    fn argument_parser_preserves_non_unicode_operands() {
        let operand = OsString::from_vec(b"diff-\xff.patch".to_vec());
        let parsed = parse_args([
            OsString::from("--paging=always"),
            OsString::from("--wrap"),
            operand,
        ])
        .unwrap();

        assert_eq!(parsed.paging, PagingMode::Always);
        assert!(parsed.wrap);
        assert_eq!(parsed.paths.len(), 1);
        assert_eq!(parsed.paths[0].as_os_str().as_bytes(), b"diff-\xff.patch");
    }
}
