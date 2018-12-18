#[cfg(feature = "log-panic")]
#[macro_use]
extern crate log;
use std::fmt::{self, Display};
use std::error::Error;
use std::panic;

/// Error type that is not supposed to be handled but reported, panicked on or ignored
#[derive(Debug)]
pub enum Problem {
    Cause(String),
    Context(String, Box<Problem>),
}

impl Problem {
    pub fn cause(msg: impl ToString) -> Problem {
        Problem::Cause(msg.to_string())
    }

    pub fn while_context(self, msg: impl ToString) -> Problem {
        Problem::Context(msg.to_string(), Box::new(self))
    }
}

impl Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Problem::Cause(ref msg) => write!(f, "{}", msg),
            &Problem::Context(ref msg, ref inner) => match inner.as_ref() {
                &Problem::Cause(ref cause) => write!(f, "while {} got problem caused by: {}", msg, cause),
                inner => write!(f, "while {}, {}", msg, inner),
            }
        }
    }
}

/// Every type implementing Error trait can implicitly be converted to Problem via ? operator
impl<E> From<E> for Problem where E: Error {
    fn from(error: E) -> Problem {
        Problem::cause(error)
    }
}

/// Explicit conversion to Problem
pub trait ToProblem {
    fn to_problem(self) -> Problem;
}

/// T that has Display or ToString implemented can be converted to Problem
impl<T> ToProblem for T where T: ToString {
    fn to_problem(self) -> Problem {
        Problem::cause(self)
    }
}

/// Option of T that has Display or ToString implemented can be converted to Problem that displays <unknown error> for None variant
pub trait OptionErrorToProblem {
    fn to_problem(self) -> Problem;
}

impl<E> OptionErrorToProblem for Option<E> where E: ToProblem {
    fn to_problem(self) -> Problem {
        self.map(ToProblem::to_problem).unwrap_or(Problem::cause("<unknown error>"))
    }
}

/// Mapping Result with error to Result with Problem
pub trait ResultToProblem<O> {
    fn map_problem(self) -> Result<O, Problem>;
}

impl<O, E> ResultToProblem<O> for Result<O, E> where E: ToProblem {
    fn map_problem(self) -> Result<O, Problem> {
        self.map_err(|e| e.to_problem())
    }
}

/// Mapping Result with Option<Error> to Result with Problem
pub trait ResultOptionToProblem<O> {
    fn map_problem(self) -> Result<O, Problem>;
}

impl<O, E> ResultOptionToProblem<O> for Result<O, Option<E>> where E: ToProblem {
    fn map_problem(self) -> Result<O, Problem> {
        self.map_err(OptionErrorToProblem::to_problem)
    }
}

/// Map Option to Result with Problem
pub trait OptionToProblem<O> {
    fn ok_or_problem<M>(self, msg: M) -> Result<O, Problem> where M: ToString;
}

impl<O> OptionToProblem<O> for Option<O> {
    fn ok_or_problem<M>(self, msg: M) -> Result<O, Problem> where M: ToString {
        self.ok_or_else(|| Problem::cause(msg))
    }
}

/// Add context to Result with Problem or that can be implicitly mapped to one
pub trait ProblemWhile<O> {
    fn problem_while(self, msg: impl Display) -> Result<O, Problem>;
    fn problem_while_with<F, M>(self, msg: F) -> Result<O, Problem> where F: FnOnce() -> M, M: Display;
}

impl<O, E> ProblemWhile<O> for Result<O, E> where E: Into<Problem> {
    fn problem_while(self, msg: impl Display) -> Result<O, Problem> {
        self.map_err(|err| err.into().while_context(msg))
    }

    fn problem_while_with<F, M>(self, msg: F) -> Result<O, Problem> where F: FnOnce() -> M, M: Display {
        self.map_err(|err| err.into().while_context(msg()))
    }
}

/// Executes closure with problem_while context
pub fn in_context_of<O, M, B>(msg: M, body: B) -> Result<O, Problem> where M: Display, B: FnOnce() -> Result<O, Problem> {
    body().problem_while(msg)
}

/// Executes closure with problem_while_with context
pub fn in_context_of_with<O, F, M, B>(msg: F, body: B) -> Result<O, Problem> where F: FnOnce() -> M, M: Display, B: FnOnce() -> Result<O, Problem> {
    body().problem_while_with(msg)
}

/// Extension of Result that allows program to panic with Display message on Err for fatal application errors that are not bugs
pub trait FailedTo<O> {
    fn or_failed_to(self, msg: impl Display) -> O;
}

impl<O, E> FailedTo<O> for Result<O, E> where E: Display {
    fn or_failed_to(self, msg: impl Display) -> O {
        match self {
            Err(err) => panic!("Failed to {} due to: {}", msg, err),
            Ok(ok) => ok
        }
    }
}

impl<O> FailedTo<O> for Option<O> {
    fn or_failed_to(self, msg: impl Display) -> O {
        match self {
            None => panic!("Failed to {}", msg),
            Some(ok) => ok
        }
    }
}

/// Iterator that will panic on first error with message displaying Display formatted message
pub struct ProblemIter<I> {
    inner: I,
    message: String
}

impl<I, O, E> Iterator for ProblemIter<I> where I: Iterator<Item=Result<O, E>>, E: Display  {
    type Item = O;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|res| res.or_failed_to(self.message.as_str()))
    }
}

/// Convert Iterator of Result<O, E> to iterator of O and panic on first E with problem message
pub trait FailedToIter<O, E>: Sized {
    fn or_failed_to(self, msg: impl ToString) -> ProblemIter<Self>;
}

impl<I, O, E> FailedToIter<O, E> for I where I: Iterator<Item=Result<O, E>>, E: Display {
    fn or_failed_to(self, msg: impl ToString) -> ProblemIter<Self> {
        ProblemIter {
            inner: self,
            message: msg.to_string()
        }
    }
}

/// Set panic hook so that when program panics it the Display version of error massage will be printed to stderr
pub fn format_panic_to_stderr() {
    panic::set_hook(Box::new(|panic_info| {
        if let Some(value) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("{}", value);
        } else if let Some(value) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("{}", value);
        } else if let Some(value) = panic_info.payload().downcast_ref::<&Error>() {
            eprintln!("{}", value);
        } else {
            eprintln!("Got panic with unsupported type: {:?}", panic_info);
        }
    }));
}

/// Set panic hook so that when program panics it the Display version of error massage will be logged with error! macro
#[cfg(feature = "log-panic")]
pub fn format_panic_to_error_log() {
    panic::set_hook(Box::new(|panic_info| {
        if let Some(value) = panic_info.payload().downcast_ref::<String>() {
            error!("{}", value);
        } else if let Some(value) = panic_info.payload().downcast_ref::<&str>() {
            error!("{}", value);
        } else if let Some(value) = panic_info.payload().downcast_ref::<&Error>() {
            error!("{}", value);
        } else {
            error!("Got panic with unsupported type: {:?}", panic_info);
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    #[should_panic(expected = "Failed to complete processing task due to: while processing object, while processing input data, while parsing input got problem caused by: boom!")]
    fn test_integration() {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))
            .problem_while("parsing input")
            .problem_while("processing input data")
            .problem_while("processing object")
            .or_failed_to("complete processing task")
    }

    #[test]
    #[should_panic(expected = "Failed to complete processing task due to: while doing stuff got problem caused by: boom!")]
    fn test_in_context_of() {
        in_context_of("doing stuff", || Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))?)
            .or_failed_to("complete processing task")
    }

    #[test]
    #[should_panic(expected = "Failed to foo due to: boom!")]
    fn test_result() {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))
            .or_failed_to("foo")
    }

    #[test]
    #[should_panic(expected = "Failed to foo")]
    fn test_option() {
        None
            .or_failed_to("foo")
    }

    #[test]
    #[should_panic(expected = "Failed to foo due to: boom!")]
    fn test_option_errors() {
        Err(Some(io::Error::new(io::ErrorKind::InvalidInput, "boom!")))
            .map_problem()
            .or_failed_to("foo")
    }

    #[test]
    #[should_panic(expected = "Failed to foo due to: <unknown error>")]
    fn test_result_option_errors_unknown() {
        let err: Result<(), Option<io::Error>> = Err(None);
        err
            .map_problem()
            .or_failed_to("foo")
    }

    #[test]
    #[should_panic(expected = "Failed to foo due to: nothing here")]
    fn test_result_ok_or_problem() {
        None
            .ok_or_problem("nothing here")
            .or_failed_to("foo")
    }

    #[test]
    #[should_panic(expected = "Failed to foo due to: omg!")]
    fn test_result_iter_or_failed_to() {
        let results = vec![Ok(1u32), Ok(2u32), Err("omg!")];
        let _ok = results.into_iter()
            .or_failed_to("foo")
            .collect::<Vec<_>>();
    }
}
