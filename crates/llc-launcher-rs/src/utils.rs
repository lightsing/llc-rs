use eyre::{EyreHandler, InstallError};
use indenter::indented;
use llc_rs::utils::ResultExt;
use std::{
    backtrace::Backtrace, collections::VecDeque, error::Error, fmt::Formatter, iter, sync::Mutex,
};

static LAST_ERRORS: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

pub fn next_error() -> Option<String> {
    LAST_ERRORS.lock().infallible().front().cloned()
}

pub fn consume_next_error() -> Option<String> {
    LAST_ERRORS.lock().infallible().pop_front()
}

/// A custom context type for capturing backtraces on stable with `eyre`
#[derive(Debug)]
struct Handler {
    backtrace: Backtrace,
}

impl EyreHandler for Handler {
    fn debug(&self, error: &(dyn Error + 'static), f: &mut Formatter<'_>) -> core::fmt::Result {
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

fn hook(_e: &(dyn Error + 'static)) -> Box<dyn EyreHandler> {
    let handler = Handler {
        backtrace: Backtrace::force_capture(),
    };

    let mut error_message = String::new();
    let mut formatter = Formatter::new(&mut error_message, Default::default());
    handler.debug(_e, &mut formatter).ok();

    LAST_ERRORS.lock().infallible().push_back(error_message);

    Box::new(handler)
}

fn is_noisy_backtrace_line(line: &str) -> bool {
    line.contains("core::")
        || line.contains("std::")
        || line.contains("tokio::")
        || line.contains("eyre")
}

/// Install the given hook as the global error report hook
pub fn install_eyre_hook() -> Result<(), InstallError> {
    eyre::set_hook(Box::new(hook))
}
