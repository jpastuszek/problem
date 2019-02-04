/*!
This crate introduces `Problem` type which can be used on high level APIs (e.g. in command line program) for which error handling boils down to:
* reporting error message (e.g. log with `error!` macro),
* aborting program on error other than a bug (e.g. using `panic!` macro),
* ignoring error.

# Problem type
`Problem` type is core of this library. It is basically a wrapper around `String`.
In order to support conversion from types implementing `Error` trait it does not implement this trait.
When converting other errors to `Problem` the `Display` message is produced of the original error and stored in `Problem` as cause message.
Additionally `Problem` can also store message and another `Problem` which allows for nesting multiple contexts and problem causes.

# Creating Problem
There are multiple ways to crate `Problem` value.

## Directly
Using `Problem::cause(msg)` function.

```rust
use problem::prelude::*;

Problem::cause("foo");
```

## Implicitly
Types implementing `Error` trait can be converted to `Problem` via `From` trait so that `?` will work.

```rust
use problem::prelude::*;

fn foo() -> Result<String, Problem> {
    let str = String::from_utf8(vec![0, 123, 255])?;
    Ok(str)
}
assert_eq!(foo().unwrap_err().to_string(), "invalid utf-8 sequence of 1 bytes from index 2");
```

If Error cause or source is available it will be shown as well.
```rust
use problem::prelude::*;
use std::fmt;
use std::error::Error;

#[derive(Debug)]
struct ErrorWithCause(std::string::FromUtf8Error);

impl fmt::Display for ErrorWithCause {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bad things happened")
    }
}

impl Error for ErrorWithCause {
    fn cause(&self) -> Option<&dyn Error> {
        Some(&self.0)
    }
}

fn foo() -> Result<String, Problem> {
    let str = String::from_utf8(vec![0, 123, 255]).map_err(ErrorWithCause)?;
    Ok(str)
}
assert_eq!(foo().unwrap_err().to_string(), "bad things happened; caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

## Explicitly
Any type that implements `ToString` or `Display` can be converted to `Problem` with `.to_problem()`.

```rust
use problem::prelude::*;

assert_eq!("oops".to_problem().to_string(), "oops");
```

## From Option
Often when working with C libraries actual errors may be unknown and function `Result` will have `Option<impl Error>` for their `Err` variant type.
`.to_problem()` method is implemented for `Option<E>` and will contain "\<unknown error\>" message for `None` variant.

```rust
use problem::prelude::*;

let unknown: Option<&'static str> = None;
let known: Option<&'static str> = Some("oops");

assert_eq!(unknown.to_problem().to_string(), "<unknown error>");
assert_eq!(known.to_problem().to_string(), "oops");
```

## By mapping Result
`Result<T, E>` can be mapped into `Result<T, Problem>` with `.map_problem()` function.

```rust
use problem::prelude::*;

let res: Result<(), &'static str> = Err("oops");

assert_eq!(res.map_problem().unwrap_err().to_string(), "oops");
```

## By conversion of Option to Result
`Option<T>` can be converted into `Result<T, Problem>` with `.ok_or_problem(message)` function.

```rust
use problem::prelude::*;

let opt: Option<()> = None;

assert_eq!(opt.ok_or_problem("oops").unwrap_err().to_string(), "oops");
```

# Adding context to Problem

## Inline
Methods `.problem_while(message)` and `.problem_while_with(|| message)` can be called on any `Result` that error type can be implicitly converted to `Problem`.

```rust
use problem::prelude::*;

let res = String::from_utf8(vec![0, 123, 255]);

assert_eq!(res.problem_while("creating string").unwrap_err().to_string(), "while creating string got error caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

The `_with` variant can be used to delay computation of error message to the moment when actual `Err` variant has occurred.

```rust
use problem::prelude::*;

let res = String::from_utf8(vec![0, 123, 255]);

assert_eq!(res.problem_while_with(|| "creating string").unwrap_err().to_string(), "while creating string got error caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

## Wrapped
Functions `in_context_of(message, closure)` and `in_context_of_with(|| message, closure)` can be used to wrap block of code in closure that return `Result`.
This is useful when you want to add context to any error that can happen in the block of code and use `?` operator.
The return type of the closure needs to be `Result<T, Problem>`.

```rust
use problem::prelude::*;

let res = in_context_of("processing string", || {
    let _s = String::from_utf8(vec![0, 123, 255])?;
    // do some processing of _s
    Ok(())
});

assert_eq!(res.unwrap_err().to_string(), "while processing string got error caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

The `_with` variant can be used to delay computation of error message to the moment when actual `Err` variant has occurred.

```rust
use problem::prelude::*;

let res = in_context_of_with(|| "processing string", || {
    let _s = String::from_utf8(vec![0, 123, 255])?;
    // do some processing of _s
    Ok(())
});

assert_eq!(res.unwrap_err().to_string(), "while processing string got error caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

## Nested context
Context methods can be used multiple times to add another layer of context.

```rust
use problem::prelude::*;

fn foo() -> Result<String, Problem> {
    let str = String::from_utf8(vec![0, 123, 255])?;
    Ok(str)
}

let res = in_context_of("doing stuff", || {
    let _s = foo().problem_while("running foo")?;
    // do some processing of _s
    Ok(())
});

assert_eq!(res.unwrap_err().to_string(), "while doing stuff, while running foo got error caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

# Aborting program on Problem
`panic!(msg, problem)` macro can be used directly to abort program execution but error message printed on the screen will be formatted with `Debug` implementation.
This library provides function `format_panic_to_stderr()` to set up hook that will use `eprintln!("{}", message)` to report panics.

If `log` feature is enabled (default) function `format_panic_to_error_log()` will set up hook that will log with `error!("{}", message)` to report panics.

```noformat
ERROR: Panicked in libcore/slice/mod.rs:2334:5: index 18492 out of range for slice of length 512
```

## Panicking on Result with Problem
Similarly to `.expect(message)`, method `.or_failed_to(message)` can be used to abort the program via `panic!()` with `Display` formatted message when called on `Err` variant of `Result` with error type implementing `Display` trait.

```rust,should_panic
use problem::prelude::*;
use problem::format_panic_to_stderr;

format_panic_to_stderr();

// Prints message: Failed to convert string due to: invalid utf-8 sequence of 1 bytes from index 2
let _s = String::from_utf8(vec![0, 123, 255]).or_failed_to("convert string");
```

## Panicking on Option
Similarly to `.ok_or(error)`, method `.or_failed_to(message)` can be used to abort the program via `panic!()` with formatted message on `None` variant of `Option` type.

```rust,should_panic
use problem::prelude::*;
use problem::format_panic_to_stderr;

format_panic_to_stderr();
let nothing: Option<&'static str> = None;

// Prints message: Failed to get something
let _s = nothing.or_failed_to("get something");
```

## Panicking on iterators of Result
Method `.or_failed_to(message)` can be used to abort the program via `panic!()` with formatted message on iterators with `Result` item when first `Err` is encountered otherwise unwrapping the `Ok` value.

```rust,should_panic
use problem::prelude::*;
use problem::format_panic_to_stderr;

format_panic_to_stderr();

let results = vec![Ok(1u32), Ok(2u32), Err("oops")];

// Prints message: Failed to collect numbers due to: oops
let _ok: Vec<u32> = results.into_iter()
    .or_failed_to("collect numbers")
    .collect();
```

# Logging errors
If `log` feature is enabled (default) function `ok_or_log_warn()` or `ok_or_log_error()` can be used on `Result` and iterator of `Result` items to convert 
`Result` into `Option` while logging `Err` wariants as warnings or errors.
When used on iterators `flatten()` addaptor can be used to filter out all `Err` variant items after they were logged and converted to `None`.

```rust
use problem::prelude::*;

let results = vec![Ok(1u32), Ok(2), Err("oops"), Ok(3), Err("oh"), Ok(4)];

// Logs warning messages: Continuing with error oops
// Logs warning messages: Continuing with error oh
let ok: Vec<u32> = results.into_iter()
    .ok_or_log_warn()
    .flatten()
    .collect();

assert_eq!(ok.as_slice(), [1, 2, 3, 4]);
```

# Backtraces
When compiled with `backtrace` feature (default) formatting of backtraces for `Problem` cause and `panic!` locations can be enabled via `RUST_BACKTRACE=1` environment variable.

```noformat
Fatal error: Panicked in src/lib.rs:189:25: Failed to complete processing task due to: while processing object, while processing input data, while parsing input got error caused by: boom!
        --- Cause
        at backtrace::backtrace::trace_unsynchronized::h7e40b70e3b5d7257(/Users/wcc/.cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.13/src/backtrace/mod.rs:57)
        at problem::Problem::cause::h8e82f78cae379944(/Users/wcc/Documents/problem/src/lib.rs:17)
        at <problem::Problem as core::convert::From<E>>::from::h68fcd01f7485d6fd(/Users/wcc/Documents/problem/src/lib.rs:55)
        at <T as core::convert::Into<U>>::into::hf86686e788a07f6b(/Users/wcc/Documents/problem/libcore/convert.rs:456)
        at <core::result::Result<O, E> as problem::ProblemWhile<O>>::problem_while::{{closure}}::h712c2996b4c3f676(/Users/wcc/Documents/problem/src/lib.rs:147)
        at <core::result::Result<T, E>>::map_err::h6da0ba4797049470(/Users/wcc/Documents/problem/libcore/result.rs:530)
        at <core::result::Result<O, E> as problem::ProblemWhile<O>>::problem_while::h108bd26e9cdec72e(/Users/wcc/Documents/problem/src/lib.rs:147)
        at problem::tests::test_panic_format_stderr_problem::h519245df9f30ee8f(/Users/wcc/Documents/problem/src/lib.rs:412)
        at problem::tests::test_panic_format_stderr_problem::{{closure}}::haaae053a88c4688a(/Users/wcc/Documents/problem/src/lib.rs:410)
        at core::ops::function::FnOnce::call_once::h805a1d08e5489f20(/Users/wcc/Documents/problem/libcore/ops/function.rs:238)
        at <F as alloc::boxed::FnBox<A>>::call_box::h7cd9458e96c61134(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/liballoc/boxed.rs:672)
        at ___rust_maybe_catch_panic(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libpanic_unwind/lib.rs:102)
        at std::sys_common::backtrace::__rust_begin_short_backtrace::h907b48cdce2bf28d(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libtest/lib.rs:1423)
        at std::panicking::try::do_call::hffc427c76ef62020(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/panicking.rs:310)
        at ___rust_maybe_catch_panic(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libpanic_unwind/lib.rs:102)
        at <F as alloc::boxed::FnBox<A>>::call_box::h44749feefaa83f3d(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/thread/mod.rs:408)
        at std::sys_common::thread::start_thread::h24d08beb3985b9d2(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/sys_common/thread.rs:24)
        at std::sys::unix::thread::Thread::new::thread_start::h9ca5dbae56c6730a(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/sys/unix/thread.rs:90)
        at __pthread_body()
        at __pthread_start()
        --- Panicked
        at backtrace::backtrace::trace_unsynchronized::h7e40b70e3b5d7257(/Users/wcc/.cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.13/src/backtrace/mod.rs:57)
        at problem::format_panic_to_stderr::{{closure}}::h5e5215e229ccf82b(/Users/wcc/Documents/problem/src/lib.rs:319)
        at std::panicking::rust_panic_with_hook::h11860e91fb60d90b(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/panicking.rs:480)
        at std::panicking::continue_panic_fmt::h28d8e12e50184e99(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/panicking.rs:390)
        at std::panicking::begin_panic_fmt::h2bdefd173f570a0b(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/panicking.rs:345)
        at <core::result::Result<O, E> as problem::FailedTo<O>>::or_failed_to::h23df6bc9680c971b(/Users/wcc/Documents/problem/src/lib.rs:189)
        at problem::tests::test_panic_format_stderr_problem::h519245df9f30ee8f(/Users/wcc/Documents/problem/src/lib.rs:417)
        at problem::tests::test_panic_format_stderr_problem::{{closure}}::haaae053a88c4688a(/Users/wcc/Documents/problem/src/lib.rs:410)
        at core::ops::function::FnOnce::call_once::h805a1d08e5489f20(/Users/wcc/Documents/problem/libcore/ops/function.rs:238)
        at <F as alloc::boxed::FnBox<A>>::call_box::h7cd9458e96c61134(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/liballoc/boxed.rs:672)
        at ___rust_maybe_catch_panic(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libpanic_unwind/lib.rs:102)
        at std::sys_common::backtrace::__rust_begin_short_backtrace::h907b48cdce2bf28d(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libtest/lib.rs:1423)
        at std::panicking::try::do_call::hffc427c76ef62020(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/panicking.rs:310)
        at ___rust_maybe_catch_panic(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libpanic_unwind/lib.rs:102)
        at <F as alloc::boxed::FnBox<A>>::call_box::h44749feefaa83f3d(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/thread/mod.rs:408)
        at std::sys_common::thread::start_thread::h24d08beb3985b9d2(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/sys_common/thread.rs:24)
        at std::sys::unix::thread::Thread::new::thread_start::h9ca5dbae56c6730a(/rustc/abe02cefd6cd1916df62ad7dc80161bea50b72e8/src/libstd/sys/unix/thread.rs:90)
        at __pthread_body()
        at __pthread_start()
```

## Access
Formatted backtrace `&str` can be accessed via `Problem::backtrace` function that will return `Some` if `backtrace` feature is available and `RUST_BACKTRACE=1` environment variable is set.

```rust
use problem::prelude::*;

Problem::cause("foo").backtrace(); // Some(<&str>)
```
 */
#[cfg(feature = "log")]
#[macro_use]
extern crate log;
use std::error::Error;
use std::fmt::{self, Display};
use std::panic;

/// Includes `Problem` type and related conversion traits and `in_context_of*` functions
pub mod prelude {
    pub use super::{
        in_context_of, in_context_of_with, FailedTo, FailedToIter, OptionErrorToProblem,
        OptionToProblem, Problem, ProblemWhile, ResultOptionToProblem, ResultToProblem, ToProblem,
    };

    #[cfg(feature = "log")]
    pub use super::logged::{OkOrLog, OkOrLogIter};
}

/// Error type that is not supposed to be handled but reported, panicked on or ignored
#[derive(Debug)]
pub enum Problem {
    Cause(String, Option<String>),
    Context(String, Box<Problem>),
}

impl Problem {
    pub fn cause(msg: impl ToString) -> Problem {
        Problem::Cause(msg.to_string(), format_backtrace())
    }

    pub fn while_context(self, msg: impl ToString) -> Problem {
        Problem::Context(msg.to_string(), Box::new(self))
    }

    pub fn backtrace(&self) -> Option<&str> {
        match self {
            &Problem::Cause(_, ref backtrace) => backtrace.as_ref().map(String::as_str),
            &Problem::Context(_, ref problem) => problem.backtrace(),
        }
    }
}

impl Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Problem::Cause(ref msg, None) => write!(f, "{}", msg),
            &Problem::Cause(ref msg, Some(ref backtrace)) => {
                write!(f, "{}\n\t--- Cause\n{}", msg, backtrace)
            }
            &Problem::Context(ref msg, ref inner) => match inner.as_ref() {
                cause @ &Problem::Cause(..) => {
                    write!(f, "while {} got error caused by: {}", msg, cause)
                }
                inner => write!(f, "while {}, {}", msg, inner),
            },
        }
    }
}

/// Every type implementing Error trait can implicitly be converted to Problem via ? operator
impl<E> From<E> for Problem
where
    E: Error,
{
    fn from(error: E) -> Problem {
        let message = if let Some(cause) = error.cause() {
            format!("{}; caused by: {}", error.to_string(), cause.to_string())
        } else {
            error.to_string()
        };
        Problem::cause(message)
    }
}

/// Explicit conversion to Problem
pub trait ToProblem {
    fn to_problem(self) -> Problem;
}

/// T that has Display or ToString implemented can be converted to Problem
impl<T> ToProblem for T
where
    T: ToString,
{
    fn to_problem(self) -> Problem {
        Problem::cause(self)
    }
}

/// Option of T that has Display or ToString implemented can be converted to Problem that displays <unknown error> for None variant
pub trait OptionErrorToProblem {
    fn to_problem(self) -> Problem;
}

impl<E> OptionErrorToProblem for Option<E>
where
    E: ToProblem,
{
    fn to_problem(self) -> Problem {
        self.map(ToProblem::to_problem)
            .unwrap_or(Problem::cause("<unknown error>"))
    }
}

/// Mapping Result with error to Result with Problem
pub trait ResultToProblem<O> {
    fn map_problem(self) -> Result<O, Problem>;
}

impl<O, E> ResultToProblem<O> for Result<O, E>
where
    E: ToProblem,
{
    fn map_problem(self) -> Result<O, Problem> {
        self.map_err(|e| e.to_problem())
    }
}

/// Mapping Result with Option<Error> to Result with Problem
pub trait ResultOptionToProblem<O> {
    fn map_problem(self) -> Result<O, Problem>;
}

impl<O, E> ResultOptionToProblem<O> for Result<O, Option<E>>
where
    E: ToProblem,
{
    fn map_problem(self) -> Result<O, Problem> {
        self.map_err(OptionErrorToProblem::to_problem)
    }
}

/// Map Option to Result with Problem
pub trait OptionToProblem<O> {
    fn ok_or_problem<M>(self, msg: M) -> Result<O, Problem>
    where
        M: ToString;
}

impl<O> OptionToProblem<O> for Option<O> {
    fn ok_or_problem<M>(self, msg: M) -> Result<O, Problem>
    where
        M: ToString,
    {
        self.ok_or_else(|| Problem::cause(msg))
    }
}

/// Add context to Result with Problem or that can be implicitly mapped to one
pub trait ProblemWhile<O> {
    fn problem_while(self, msg: impl Display) -> Result<O, Problem>;
    fn problem_while_with<F, M>(self, msg: F) -> Result<O, Problem>
    where
        F: FnOnce() -> M,
        M: Display;
}

impl<O, E> ProblemWhile<O> for Result<O, E>
where
    E: Into<Problem>,
{
    fn problem_while(self, msg: impl Display) -> Result<O, Problem> {
        self.map_err(|err| err.into().while_context(msg))
    }

    fn problem_while_with<F, M>(self, msg: F) -> Result<O, Problem>
    where
        F: FnOnce() -> M,
        M: Display,
    {
        self.map_err(|err| err.into().while_context(msg()))
    }
}

/// Executes closure with problem_while context
pub fn in_context_of<O, M, B>(msg: M, body: B) -> Result<O, Problem>
where
    M: Display,
    B: FnOnce() -> Result<O, Problem>,
{
    body().problem_while(msg)
}

/// Executes closure with problem_while_with context
pub fn in_context_of_with<O, F, M, B>(msg: F, body: B) -> Result<O, Problem>
where
    F: FnOnce() -> M,
    M: Display,
    B: FnOnce() -> Result<O, Problem>,
{
    body().problem_while_with(msg)
}

/// Extension of Result that allows program to panic with Display message on Err for fatal application errors that are not bugs
pub trait FailedTo<O> {
    fn or_failed_to(self, msg: impl Display) -> O;
}

impl<O, E> FailedTo<O> for Result<O, E>
where
    E: Display,
{
    fn or_failed_to(self, msg: impl Display) -> O {
        match self {
            Err(err) => panic!("Failed to {} due to: {}", msg, err),
            Ok(ok) => ok,
        }
    }
}

impl<O> FailedTo<O> for Option<O> {
    fn or_failed_to(self, msg: impl Display) -> O {
        match self {
            None => panic!("Failed to {}", msg),
            Some(ok) => ok,
        }
    }
}

/// Iterator that will panic on first error with message displaying Display formatted message
pub struct ProblemIter<I> {
    inner: I,
    message: String,
}

impl<I, O, E> Iterator for ProblemIter<I>
where
    I: Iterator<Item = Result<O, E>>,
    E: Display,
{
    type Item = O;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|res| res.or_failed_to(self.message.as_str()))
    }
}

/// Convert Iterator of Result<O, E> to iterator of O and panic on first E with problem message
pub trait FailedToIter<O, E>: Sized {
    fn or_failed_to(self, msg: impl ToString) -> ProblemIter<Self>;
}

impl<I, O, E> FailedToIter<O, E> for I
where
    I: Iterator<Item = Result<O, E>>,
    E: Display,
{
    fn or_failed_to(self, msg: impl ToString) -> ProblemIter<Self> {
        ProblemIter {
            inner: self,
            message: msg.to_string(),
        }
    }
}

#[cfg(feature = "log")]
pub mod logged {
    use super::*;
    use log::{warn, error};

    /// Extension of Result that allows program to log on Err with Display message for application errors that are not critical
    pub trait OkOrLog<O> {
        fn ok_or_log_warn(self) -> Option<O>;
        fn ok_or_log_error(self) -> Option<O>;
    }

    impl<O, E> OkOrLog<O> for Result<O, E>
    where
        E: Display,
    {
        fn ok_or_log_warn(self) -> Option<O> {
            match self {
                Err(err) => {
                    warn!("Continuing with error {}", err);
                    None
                },
                Ok(ok) => Some(ok),
            }
        }

        fn ok_or_log_error(self) -> Option<O> {
            match self {
                Err(err) => {
                    error!("Continuing with error {}", err);
                    None
                },
                Ok(ok) => Some(ok),
            }
        }
    }

    /// Iterator that will log as warn Display formatted message on Err and skip to next item; it can be flattened to skip failed items
    pub struct ProblemWarnLoggingIter<I> {
        inner: I,
    }

    impl<I, O, E> Iterator for ProblemWarnLoggingIter<I>
    where
        I: Iterator<Item = Result<O, E>>,
        E: Display,
    {
        type Item = Option<O>;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner
                .next()
                .map(|res| res.ok_or_log_warn())
        }
    }

    /// Iterator that will log as error Display formatted message on Err and skip to next item; it can be flattened to skip failed items
    pub struct ProblemErrorLoggingIter<I> {
        inner: I,
    }

    impl<I, O, E> Iterator for ProblemErrorLoggingIter<I>
    where
        I: Iterator<Item = Result<O, E>>,
        E: Display,
    {
        type Item = Option<O>;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner
                .next()
                .map(|res| res.ok_or_log_error())
        }
    }

    /// Convert Iterator of Result<O, E> to iterator of Option<O> and log any Err variants
    pub trait OkOrLogIter<O, E>: Sized {
        fn ok_or_log_warn(self) -> ProblemWarnLoggingIter<Self>;
        fn ok_or_log_error(self) -> ProblemErrorLoggingIter<Self>;
    }

    impl<I, O, E> OkOrLogIter<O, E> for I
    where
        I: Iterator<Item = Result<O, E>>,
        E: Display,
    {
        fn ok_or_log_warn(self) -> ProblemWarnLoggingIter<Self> {
            ProblemWarnLoggingIter {
                inner: self,
            }
        }

        fn ok_or_log_error(self) -> ProblemErrorLoggingIter<Self> {
            ProblemErrorLoggingIter {
                inner: self,
            }
        }
    }
}

#[cfg(not(feature = "backtrace"))]
fn format_backtrace() -> Option<String> {
    None
}

#[cfg(feature = "backtrace")]
#[inline(always)]
fn format_backtrace() -> Option<String> {
    if let Ok("1") = std::env::var("RUST_BACKTRACE").as_ref().map(String::as_str) {
        let mut backtrace = String::new();

        backtrace::trace(|frame| {
            let ip = frame.ip();
            //let symbol_address = frame.symbol_address();

            backtrace.push_str("\tat ");

            backtrace::resolve(ip, |symbol| {
                if let Some(name) = symbol.name() {
                    backtrace.push_str(&name.to_string());
                }
                backtrace.push_str("(");
                if let Some(filename) = symbol.filename() {
                    backtrace.push_str(&filename.display().to_string());
                }
                if let Some(lineno) = symbol.lineno() {
                    backtrace.push_str(":");
                    backtrace.push_str(&lineno.to_string());
                }
                backtrace.push_str(")");
            });

            backtrace.push_str("\n");
            true // keep going to the next frame
        });

        backtrace.pop(); // last \n

        Some(backtrace)
    } else {
        None
    }
}

fn format_panic(panic: &std::panic::PanicInfo, backtrace: Option<String>) -> String {
    let mut message = String::new();

    if let Some(location) = panic.location() {
        message.push_str(&format!(
            "Panicked in {}:{}:{}: ",
            location.file(),
            location.line(),
            location.column()
        ));
    };

    if let Some(value) = panic.payload().downcast_ref::<String>() {
        message.push_str(&value);
    } else if let Some(value) = panic.payload().downcast_ref::<&str>() {
        message.push_str(value);
    } else if let Some(value) = panic.payload().downcast_ref::<&Error>() {
        message.push_str(&format!("{} ({:?})", value, value));
    } else {
        message.push_str(&format!("{:?}", panic));
    };

    if let Some(backtrace) = backtrace {
        message.push_str("\n\t--- Panicked\n");
        message.push_str(&backtrace);
    };

    message
}

/// Set panic hook so that when program panics it will print the Display version of error massage to stderr
pub fn format_panic_to_stderr() {
    panic::set_hook(Box::new(|panic_info| {
        let backtrace = format_backtrace();
        eprintln!("Fatal error: {}", format_panic(panic_info, backtrace));
    }));
}

/// Set panic hook so that when program panics it will log the Display version of error massage with error! macro
#[cfg(feature = "log")]
pub fn format_panic_to_error_log() {
    panic::set_hook(Box::new(|panic_info| {
        let backtrace = format_backtrace();
        error!("{}", format_panic(panic_info, backtrace));
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    #[should_panic(
        expected = "Failed to complete processing task due to: while processing object, while processing input data, while parsing input got error caused by: boom!"
    )]
    fn test_integration() {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))
            .problem_while("parsing input")
            .problem_while("processing input data")
            .problem_while("processing object")
            .or_failed_to("complete processing task")
    }

    #[test]
    #[should_panic(
        expected = "Failed to complete processing task due to: while doing stuff got error caused by: boom!"
    )]
    fn test_in_context_of() {
        in_context_of("doing stuff", || {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))?
        })
        .or_failed_to("complete processing task")
    }

    #[test]
    #[should_panic(expected = "Failed to foo due to: boom!")]
    fn test_result() {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!")).or_failed_to("foo")
    }

    #[test]
    #[should_panic(expected = "Failed to foo")]
    fn test_option() {
        None.or_failed_to("foo")
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
        err.map_problem().or_failed_to("foo")
    }

    #[test]
    #[should_panic(expected = "Failed to foo due to: nothing here")]
    fn test_result_ok_or_problem() {
        None.ok_or_problem("nothing here").or_failed_to("foo")
    }

    #[test]
    #[should_panic(expected = "Failed to foo due to: omg!")]
    fn test_result_iter_or_failed_to() {
        let results = vec![Ok(1u32), Ok(2u32), Err("omg!")];
        let _ok = results.into_iter().or_failed_to("foo").collect::<Vec<_>>();
    }

    #[test]
    #[should_panic]
    fn test_panic_format_stderr() {
        format_panic_to_stderr();
        panic!("foo bar!");
    }

    #[test]
    #[should_panic]
    fn test_panic_format_stderr_problem() {
        format_panic_to_stderr();
        let result: Result<(), Problem> = Err(io::Error::new(io::ErrorKind::InvalidInput, "boom!"))
            .problem_while("parsing input")
            .problem_while("processing input data")
            .problem_while("processing object");

        result.or_failed_to("complete processing task");
    }

    #[test]
    #[cfg(feature = "backtrace")]
    fn test_problem_backtrace() {
        let p = Problem::cause("foo")
            .while_context("bar")
            .while_context("baz");

        if let Ok("1") = std::env::var("RUST_BACKTRACE").as_ref().map(String::as_str) {
            assert!(p.backtrace().is_some());
            println!("{}", p.backtrace().unwrap());
        } else {
            assert!(p.backtrace().is_none());
        }
    }
}
