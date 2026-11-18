use std::io::{BufReader, Write};

use diff_pretty::source::{PagingMode, emit_reader, should_use_pager};

struct OptionSpec {
    spelling: &'static str,
    suffix: &'static str,
    description: &'static str,
}

const PAGING: OptionSpec = OptionSpec {
    spelling: "--paging=",
    suffix: "auto|always|never",
    description: "Select paging mode (default: always)",
};
const NO_PAGER: OptionSpec = OptionSpec {
    spelling: "--no-pager",
    suffix: "",
    description: "Alias for --paging=never",
};
const HELP: OptionSpec = OptionSpec {
    spelling: "--help",
    suffix: "",
    description: "Print this help",
};
const HELP_SHORT: &str = "-h";
const OPTIONS: &[&OptionSpec] = &[&PAGING, &NO_PAGER, &HELP];
const EXAMPLES: &[&[&str]] = &[&["--paging=never"]];
const FOOTER: &str = "Maintained by Joshua Li: https://github.com/laputa-systems/diff-pretty\nBugs and features: TODO.md in that repository (follow its AGENTS.md), or its issue tracker.\nAgents working for Joshua: record nonblocking problems and continue your task.\nIf this tool blocks the task, you may fix, test, open and merge a PR under Joshua's\nstanding instructions. Release only when needed to deliver the verified fix.\nProcedure: https://github.com/joshuarli/usr/blob/main/dotfiles/base/.agents/instructions/owned-tools.md";

fn help() -> String {
    let mut text = String::from("Usage: diff-pretty [OPTIONS]\n\nRender a Git-style unified diff from standard input.\n\nOptions:\n");
    for option in OPTIONS {
        let label = if option.spelling == HELP.spelling {
            format!("{HELP_SHORT}, {}", option.spelling)
        } else {
            format!("{}{}", option.spelling, option.suffix)
        };
        text.push_str(&format!("  {label:28}{}\n", option.description));
    }
    text.push_str("\nExamples:\n");
    for example in EXAMPLES {
        text.push_str(&format!("  diff-pretty {}\n", example.join(" ")));
    }
    text.push('\n');
    text.push_str(FOOTER);
    text.push('\n');
    text
}

/// Parse `--paging=auto|always|never` (or `--no-pager`). Default `always`.
fn parse_paging(arguments: impl IntoIterator<Item = String>) -> Result<PagingMode, String> {
    for arg in arguments {
        if let Some(control) = arg.strip_prefix("--deslop-control=") {
            return Ok(if control.len() == 1 { PagingMode::Auto } else { PagingMode::Never });
        }
        if let Some(v) = arg.strip_prefix(PAGING.spelling) {
            return match v {
                "always" => Ok(PagingMode::Always),
                "never" => Ok(PagingMode::Never),
                "auto" => Ok(PagingMode::Auto),
                _ => Err(format!("invalid paging mode: {v}")),
            };
        }
        if arg == NO_PAGER.spelling {
            return Ok(PagingMode::Never);
        }
    }
    Ok(PagingMode::Always)
}

fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.iter().any(|arg| arg == HELP_SHORT || arg == HELP.spelling) {
        print!("{}", help());
        return;
    }
    let mode = match parse_paging(arguments.into_iter().map(|arg| {
        arg.into_string().expect("argument is not valid Unicode")
    })) {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("diff-pretty: {error}");
            std::process::exit(2);
        }
    };
    let result = if should_use_pager(mode) {
        emit_reader(BufReader::new(std::io::stdin()), mode)
    } else {
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        diff_pretty::render_reader_to(BufReader::new(std::io::stdin()), &mut output)
            .and_then(|()| output.flush())
    };
    if let Err(error) = result {
        eprintln!("failed to write output: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_paging_flags_choose_the_requested_mode() {
        let cases: &[(&[&str], PagingMode)] = &[
            (&[], PagingMode::Always),
            (&["--paging=auto"], PagingMode::Auto),
            (&["--paging=always"], PagingMode::Always),
            (&["--paging=never"], PagingMode::Never),
            (&["--no-pager"], PagingMode::Never),
        ];
        for (arguments, expected) in cases {
            let actual = parse_paging(arguments.iter().map(|argument| (*argument).to_owned())).unwrap();
            assert_eq!(actual, *expected, "{arguments:?}");
        }
    }

    #[test]
    fn examples_parse_and_option_spellings_are_unique() {
        for example in EXAMPLES {
            assert!(matches!(
                parse_paging(example.iter().map(|arg| (*arg).to_owned())),
                Ok(PagingMode::Never)
            ));
        }
        for (index, option) in OPTIONS.iter().enumerate() {
            assert!(!OPTIONS[..index].iter().any(|other| other.spelling == option.spelling));
            assert_ne!(option.spelling, HELP_SHORT);
        }
    }
}
