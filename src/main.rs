use std::io::{BufReader, Write};

use diff_pretty::source::{PagingMode, emit_reader, should_use_pager};

/// Parse `--paging=auto|always|never` (or `--no-pager`). Default `always`.
fn parse_paging(arguments: impl IntoIterator<Item = String>) -> Result<PagingMode, String> {
    for arg in arguments {
        if let Some(v) = arg.strip_prefix("--paging=") {
            return match v {
                "always" => Ok(PagingMode::Always),
                "never" => Ok(PagingMode::Never),
                "auto" => Ok(PagingMode::Auto),
                _ => Err(format!("invalid paging mode: {v}")),
            };
        }
        if arg == "--no-pager" {
            return Ok(PagingMode::Never);
        }
    }
    Ok(PagingMode::Always)
}

fn main() {
    let mode = match parse_paging(std::env::args().skip(1)) {
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
