The aim of this library is to support writing command line programs or library prototypes in Rust by simplifying error handling in program code.

It introduces `Problem` type which can be used on high level APIs for which error handling boils down to:
* reporting error message (e.g. log with `error!` macro),
* aborting program on error other than a bug (e.g. using `panic!` macro),
* ignoring error (e.g. using `Result::ok`).

# Goals
* Simplifying type signatures around error handling to one type so they compose easily.
* Allow errors to bubble up easily by elimination of life times in error types.
* Produce user friendly error messages with `Display` formatting.
* Support multiple ways to add context to error message.
* Easy to work with existing types and program flows.
* Minimize performance impact on `Ok` path.

# Non Goals
* Providing ability to match on particular error variant to facilitate handling of error condition.
* High performance of `Err` path - e.g. zero allocation.
* `Sync` and `Send` compatibility.

# Example
Implicit conversion to `Problem` type and context message.
```rust,skt-problem
use problem::prelude::*;

fn foo() -> Result<String, Problem> {
    let str = String::from_utf8(vec![0, 123, 255])?;
    Ok(str)
}

let result = foo().problem_while("creating string");

assert_eq!(result.unwrap_err().to_string(), "while creating string got error caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

Handling fatal errors with panic using `Problem::or_failed_to`.
```rust,should_panic,skt-problem
use problem::prelude::*;
use problem::format_panic_to_stderr;

// Replace Rust default panic handler
format_panic_to_stderr();

fn foo() -> Result<String, Problem> {
    let str = String::from_utf8(vec![0, 123, 255])?;
    Ok(str)
}

let _s = foo().or_failed_to("create a string"); // Fatal error: Panicked in src/lib.rs:464:25: Failed to create a string due to: invalid utf-8 sequence of 1 bytes from index 2
```

For more examples see crate documentation at [docs.rs](https://docs.rs/problem).