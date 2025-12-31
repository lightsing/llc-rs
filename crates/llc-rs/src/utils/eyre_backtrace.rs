use eyre::InstallError;
use indenter::indented;
use std::{backtrace::Backtrace, error::Error, iter};

/// A custom context type for capturing backtraces on stable with `eyre`
#[derive(Debug)]
struct Handler {
    backtrace: Backtrace,
}

impl eyre::EyreHandler for Handler {
    fn debug(
        &self,
        error: &(dyn Error + 'static),
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        use core::fmt::Write as _;

        if f.alternate() {
            return core::fmt::Debug::fmt(error, f);
        }

        write!(f, "{}", error)?;

        if let Some(cause) = error.source() {
            write!(f, "\n\nCaused by:")?;

            let multiple = cause.source().is_some();
            let errors = iter::successors(Some(cause), |e| (*e).source());

            for (n, error) in errors.enumerate() {
                writeln!(f)?;
                if multiple {
                    write!(indented(f).ind(n), "{}", error)?;
                } else {
                    write!(indented(f), "{}", error)?;
                }
            }
        }
        write!(f, "\nBacktrace:\n")?;

        let formatted = format!("{}", self.backtrace);
        let mut lines = formatted.lines().peekable();
        while let Some(line) = lines.next() {
            // skip noisy lines
            if is_noisy_backtrace_line(line) {
                if let Some(line) = lines.peek()
                    && line.contains("at ")
                {
                    // skip the next line too
                    lines.next();
                }
                continue;
            }
            writeln!(f, "{line}")?;
        }

        Ok(())
    }
}

fn hook(_e: &(dyn Error + 'static)) -> Box<dyn eyre::EyreHandler> {
    Box::new(Handler {
        backtrace: Backtrace::force_capture(),
    })
}

fn is_noisy_backtrace_line(line: &str) -> bool {
    line.contains("core::")
        || line.contains("std::")
        || line.contains("tokio::")
        || line.contains("eyre")
}

/// Install the given hook as the global error report hook
pub fn install() -> Result<(), InstallError> {
    eyre::set_hook(Box::new(hook))
}
