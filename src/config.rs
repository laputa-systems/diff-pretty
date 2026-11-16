//! Fixed colors, ANSI sequences, and layout primitives for rendered patches.

use std::fmt::Write as _;

/// ANSI SGR sequences used by the renderer. Adjacent segments share style
/// prefixes; transitions to plain reset the style, and styled lines end with
/// one reset.
pub mod sgr {
    pub const RESET: &str = "\x1b[0m";
    /// Blue: file decoration and hunk box.
    pub const BLUE: &str = "\x1b[34m";
    /// Gray: context ("zero") line numbers -> rgb(68,68,68).
    pub const ZERO_NUM: &str = "\x1b[38;2;68;68;68m";
    /// Dark red: minus-line number (color index 88).
    pub const MINUS_NUM: &str = "\x1b[38;5;88m";
    /// Dark green: plus-line number (color index 28).
    pub const PLUS_NUM: &str = "\x1b[38;5;28m";
    /// Red: removed content.
    pub const MINUS: &str = "\x1b[31m";
    /// Green: added content.
    pub const PLUS: &str = "\x1b[32m";
    /// Emph (bold + reverse) for changed words; color is inherited from the
    /// surrounding minus/plus base, so only the added attributes are emitted.
    pub const EMPH: &str = "\x1b[1;7m";

    pub const HORIZONTAL: &str = "\u{2500}"; // ─
    pub const DOWN_LEFT: &str = "\u{2510}"; // ┐
    pub const UP_LEFT: &str = "\u{2518}"; // ┘
    pub const VERTICAL: &str = "\u{2502}"; // │
    pub const BULLET: &str = "\u{2022}"; // •
    pub const RIGHT_ARROW: &str = "\u{27f6}  "; // ⟶  (arrow + two spaces)
}

/// Append a `{nm:^4}` / `{np:^4}` line-number cell for `line_number` (or
/// `width` spaces when `None`) directly to `out`, without a temporary String.
/// Keeps `pad_number`'s "center-right" shift for odd width/digit-count parity.
///
/// This is the allocation-free form used by the renderer's hot path (one cell
/// per hunk line, twice per line).
pub fn push_pad_number(out: &mut String, line_number: Option<usize>, width: usize) {
    let Some(n) = line_number else {
        push_spaces(out, width);
        return;
    };
    let n_width = num_digits(n);
    let pad = width.saturating_sub(n_width);
    let left = pad / 2;
    let right = pad - left;
    if pad > 0 && width % 2 != n_width % 2 {
        out.push(' ');
        push_spaces(out, left);
        let _ = write!(out, "{n}");
        push_spaces(out, right - 1);
    } else {
        push_spaces(out, left);
        let _ = write!(out, "{n}");
        push_spaces(out, right);
    }
}

fn push_spaces(out: &mut String, count: usize) {
    out.extend(std::iter::repeat_n(' ', count));
}

/// The line-numbers format `{nm:^4}` / `{np:^4}`: a centered field of width 4,
/// with a one-column right shift when the width and digit count differ in
/// parity. A missing number produces `width` spaces.
pub fn pad_number(line_number: Option<usize>, width: usize) -> String {
    let mut out = String::with_capacity(width);
    push_pad_number(&mut out, line_number, width);
    out
}

/// floor(log10(n)) + 1, with 0 treated as 1 digit.
pub fn num_digits(mut n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut len = 0;
    while n >= 10_000 {
        len += 4;
        n /= 10_000;
    }
    if n >= 1000 {
        len + 4
    } else if n >= 100 {
        len + 3
    } else if n >= 10 {
        len + 2
    } else {
        len + 1
    }
}

#[cfg(test)]
mod tests {
    use super::{num_digits, pad_number, push_pad_number};

    #[test]
    fn number_cells_match_the_documented_alignment() {
        for (number, width, expected) in [
            (None, 0, ""), (None, 4, "    "), (Some(0), 0, "0"),
            (Some(0), 4, "  0 "), (Some(1), 4, "  1 "),
            (Some(12), 4, " 12 "), (Some(123), 4, " 123"),
            (Some(1234), 4, "1234"), (Some(12345), 4, "12345"),
            (Some(1), 5, "  1  "), (Some(12), 5, "  12 "),
            (Some(123), 5, " 123 "), (Some(1), 1, "1"),
        ] {
            assert_eq!(pad_number(number, width), expected, "{number:?}, {width}");
            let mut appended = String::from("prefix:");
            push_pad_number(&mut appended, number, width);
            assert_eq!(appended, format!("prefix:{expected}"), "{number:?}, {width}");
        }
    }

    #[test]
    fn digit_counts_change_at_each_decimal_boundary() {
        assert_eq!(num_digits(0), 1);
        assert_eq!(num_digits(1), 1);
        for (power, digits) in [
            (10_u64, 2), (100, 3), (1_000, 4), (10_000, 5),
            (100_000, 6), (1_000_000, 7), (10_000_000, 8),
            (100_000_000, 9), (1_000_000_000, 10), (10_000_000_000, 11),
            (100_000_000_000, 12), (1_000_000_000_000, 13),
            (10_000_000_000_000, 14), (100_000_000_000_000, 15),
            (1_000_000_000_000_000, 16), (10_000_000_000_000_000, 17),
            (100_000_000_000_000_000, 18), (1_000_000_000_000_000_000, 19),
            (10_000_000_000_000_000_000, 20),
        ] {
            if let Ok(power) = usize::try_from(power) {
                assert_eq!(num_digits(power - 1), digits - 1, "below {power}");
                assert_eq!(num_digits(power), digits, "at {power}");
                assert_eq!(num_digits(power + 1), digits, "above {power}");
            }
        }
    }
}
