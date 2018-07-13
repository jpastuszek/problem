use std::fmt::{self, Display};
use std::error::Error;

/// Error type that is not supposed to be handled but reported, paniced on or ignored
#[derive(Debug)]
pub enum Problem {
    Cause(String),
    Context(String, Box<Problem>),
}

impl Problem {
    fn cause(msg: impl ToString) -> Problem {
        Problem::Cause(msg.to_string())
    }

    fn while_context(self, msg: impl ToString) -> Problem {
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

pub trait ResultToProblem<O> {
    fn to_problem(self) -> Result<O, Problem>;
}

impl<O, E> ResultToProblem<O> for Result<O, E> where E: ToProblem {
    fn to_problem(self) -> Result<O, Problem> {
        self.map_err(|e| e.to_problem())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Failed to complete processing task due to: while processing object, while processing input data, while parsing input got problem caused by: boom!")]
    fn test_integration() {
        use std::io;

        Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))
            .to_problem()
            .problem_while("parsing input")
            .problem_while("processing input data")
            .problem_while("processing object")
            .or_failed_to("complete processing task")
    }
}
