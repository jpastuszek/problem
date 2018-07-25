use std::fmt::{self, Display};
use std::error::Error;

/// Error type that is not supposed to be handled but reported, paniced on or ignored
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

// Expeced more implicit convertion
impl<E> From<E> for Problem where E: Error {
    fn from(error: E) -> Problem {
        Problem::cause(error)
    }
}

/// Explicit conversion from anything that has ToString and via this Display implemented
pub trait ToProblem {
    fn to_problem(self) -> Problem;
}

impl<T> ToProblem for T where T: ToString {
    fn to_problem(self) -> Problem {
        Problem::cause(self)
    }
}

pub trait OptionErrorToProblem {
    fn to_problem(self) -> Problem;
}

impl<E> OptionErrorToProblem for Option<E> where E: ToProblem {
    fn to_problem(self) -> Problem {
        self.map(ToProblem::to_problem).unwrap_or(Problem::cause("<unknown error>"))
    }
}

pub trait ResultToProblem<O> {
    fn to_problem(self) -> Result<O, Problem>;
}

impl<O, E> ResultToProblem<O> for Result<O, E> where E: ToProblem {
    fn to_problem(self) -> Result<O, Problem> {
        self.map_err(|e| e.to_problem())
    }
}

pub trait ResultOptionToProblem<O> {
    fn to_problem(self) -> Result<O, Problem>;
}

impl<O, E> ResultOptionToProblem<O> for Result<O, Option<E>> where E: ToProblem {
    fn to_problem(self) -> Result<O, Problem> {
        self.map_err(OptionErrorToProblem::to_problem)
    }
}

pub trait OptionToProblem<O> {
    fn ok_or_problem<M>(self, msg: M) -> Result<O, Problem> where M: ToString;
}

impl<O> OptionToProblem<O> for Option<O> {
    fn ok_or_problem<M>(self, msg: M) -> Result<O, Problem> where M: ToString {
        self.ok_or_else(|| Problem::cause(msg))
    }
}

pub trait ProblemWhile<O> {
    fn problem_while(self, msg: impl Display) -> Result<O, Problem>;
    fn problem_while_with<F, M>(self, msg: F) -> Result<O, Problem> where F: FnOnce() -> M, M: Display;
}

impl<O> ProblemWhile<O> for Result<O, Problem> {
    fn problem_while(self, msg: impl Display) -> Result<O, Problem> {
        self.map_err(|err| err.while_context(msg))
    }

    fn problem_while_with<F, M>(self, msg: F) -> Result<O, Problem> where F: FnOnce() -> M, M: Display {
        self.map_err(|err| err.while_context(msg()))
    }
}

/// Extension of Result that allows program to panic with Display message on Err handy for fata application errors that are not bugs
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

pub trait FailedToIter<O, E>: Sized {
    fn or_failed_to(self, msg: impl ToString) -> ProblemIter<Self>;
}

impl<I, O, E> FailedToIter<O, E> for I where I: Iterator<Item=Result<O, E>>, E: Display {
    /// Convert iterator or Result<O, E> to iterator of O and panic on first E with problme message
    fn or_failed_to(self, msg: impl ToString) -> ProblemIter<Self> {
        ProblemIter {
            inner: self,
            message: msg.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    #[should_panic(expected = "Failed to complete processing task due to: while processing object, while processing input data, while parsing input got problem caused by: boom!")]
    fn test_integration() {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))
            .to_problem()
            .problem_while("parsing input")
            .problem_while("processing input data")
            .problem_while("processing object")
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
            .map_err(OptionErrorToProblem::to_problem)
            .or_failed_to("foo")
    }

    #[test]
    #[should_panic(expected = "Failed to foo due to: <unknown error>")]
    fn test_option_errors_unknown() {
        let err: Result<(), Option<io::Error>> = Err(None);
        err
            .map_err(OptionErrorToProblem::to_problem)
            .or_failed_to("foo")
    }

    #[test]
    #[should_panic(expected = "Failed to foo due to: <unknown error>")]
    fn test_result_option_errors_unknown() {
        let err: Result<(), Option<io::Error>> = Err(None);
        err
            .to_problem()
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
