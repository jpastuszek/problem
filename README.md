[![Latest Version]][crates.io] [![Documentation]][docs.rs] ![License]

The aim of this library is to help writing command line programs or prototypes more efficiently in Rust by simplifying error handling in program code.

This crate introduces `Problem` type which can be used on high level APIs for which error handling boils down to:
* reporting error message (e.g. log with `error!` macro),
* aborting program on error other than a bug (e.g. using `panic!` macro),
* bubbling up errors (e.g. with `?`),
* ignoring errors (e.g. using `Result::ok`).

# Goals
* Simplify signatures of functions that can fail to one error type so they compose easier.
* Produce detailed user friendly error messages with `Display` formatting including error cause chain and backtrace (when requested).
* Make it convenient to add context to error message in different situations.
* Make it convenient to report (print or log) or abort program on error in different situations.
* Make it easy to construct `Problem` ad-hock (e.g. from string literal).
* Low effort integration with existing error types and program flows.
* Minimize performance impact on good path.
* Allow errors to bubble up by elimination of lifetimes in external error types.

# Non-goals
* Providing ability to match on particular error variant to facilitate recovering from error condition (casting).
* Zero cost error path - e.g. no allocation.

# Example
Implicit conversion to `Problem` type and context message.

```rust
use problem::prelude::*;

fn foo() -> Result<String, Problem> {
    let str = String::from_utf8(vec![0, 123, 255])?;
    Ok(str)
}

let result = foo().problem_while("creating string");

assert_eq!(result.unwrap_err().to_string(), "while creating string got error caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

Handling fatal errors with panic using `.or_failed_to()`.

```rust
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

If you are looking for fully typed, zero-cost way of adding context to error values see [error-context crate](https://github.com/jpastuszek/error-context).

[crates.io]: https://crates.io/crates/problem
[Latest Version]: https://img.shields.io/crates/v/problem.svg
[Documentation]: https://docs.rs/problem/badge.svg
[docs.rs]: https://docs.rs/problem
[License]: https://img.shields.io/crates/l/problem.svg
