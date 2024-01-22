use nom::error::{VerboseError, VerboseErrorKind};
use nom::Offset;

/// Transforms a `VerboseError` into a trace with input position information
/// Copy, pasted, overridden from nom::error::convert_error to return single
/// line errors which are more redis friendly.
pub fn convert_error<I: core::ops::Deref<Target = str>>(
  input: I,
  e: VerboseError<I>,
) -> std::string::String {
  use std::fmt::Write;

  let mut result = std::string::String::new();

  for (i, (substring, kind)) in e.errors.iter().enumerate() {
    let offset = input.offset(substring);

    if input.is_empty() {
      match kind {
        VerboseErrorKind::Char(c) => {
          write!(&mut result, "{}: expected '{}', got empty input", i, c)
        }
        VerboseErrorKind::Context(s) => write!(&mut result, "{}: in {}, got empty input ", i, s),
        VerboseErrorKind::Nom(e) => write!(&mut result, "{}: in {:?}, got empty input ", i, e),
      }
    } else {
      let prefix = &input.as_bytes()[..offset];

      // Count the number of newlines in the first `offset` bytes of input
      let line_number = prefix.iter().filter(|&&b| b == b'\n').count() + 1;

      // Find the line that includes the subslice:
      // Find the *last* newline before the substring starts
      let line_begin = prefix
        .iter()
        .rev()
        .position(|&b| b == b'\n')
        .map(|pos| offset - pos)
        .unwrap_or(0);

      // Find the full line after that newline
      let line = input[line_begin..]
        .lines()
        .next()
        .unwrap_or(&input[line_begin..])
        .trim_end();

      let error_at_index = line.offset(substring);
      let trimmed_line = input[error_at_index..]
        .lines()
        .next()
        .unwrap_or(&input[line_begin..])
        .trim_end();

      match kind {
        VerboseErrorKind::Char(c) => {
          if let Some(actual) = substring.chars().next() {
            write!(
              &mut result,
              "[{i}]: where expected '{expected}', found {actual} at '{trimmed_line}' ",
              i = i,
              trimmed_line = trimmed_line,
              expected = c,
              actual = actual,
            )
          } else {
            write!(
              &mut result,
              "[{i}]: where expected '{expected}', got end of input at '{trimmed_line}' ",
              i = i,
              trimmed_line = trimmed_line,
              expected = c,
            )
          }
        }
        VerboseErrorKind::Context(s) => write!(
            &mut result,
            "[{i}]: {context} at '{trimmed_line}' ",
            i = i,
            trimmed_line = trimmed_line,
            context = s,
        ),
        VerboseErrorKind::Nom(e) => write!(
          &mut result,
          "[{i}]: in {nom_err:?} at '{trimmed_line}' ",
          i = i,
          trimmed_line = trimmed_line,
          nom_err = e,
        ),
      }
    }
    // Because `write!` to a `String` is infallible, this `unwrap` is fine.
    .unwrap();
  }

  result
}
