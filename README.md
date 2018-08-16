The aim of this library is to support writing command line programs in Rust by simplifying error handling in program code.

It introduces `Problem` type which can be used on high level APIs for which error handling boils down to:
* reporting error message (e.g. log with `error!` macro),
* aborting program on error other than a bug (e.g. using `panic!` macro),
* ignoring error.

# Goals
* Simplifying type signatures around error handling to one type so they compose easily
* Allow errors to bubble up easily by elimination of life times in error types
* Produce user friendly error messages with `Display` formatting
* Support multiple ways to add context to error message
* Easy to work with existing types and patterns
* No performance impact on `Ok` path

# Non Goals
* Providing ability to match on particular error variant to facilitate handling of error condition
* High performance of `Err` path - e.g. zero allocation
* `Sync` and `Send` compatibility

# Problem type
`Problem` type is core of this library. It is basically a wrapper around `String`. 
In order to support conversion from types implementing `Error` trait it does not implement this trait.
When converting other errors to `Problem` the `Display` message is produced of the original error and stored in `Problem` as cause message.
Additionally `Problem` can also store message and another `Problme` which allows for nesting multiple contexts and problem causes.

# Crating Problem
There are multiple ways of crating `Problem` type.

## Directly
Using `Problem::cause(msg)` function.

```rust,skt-problem
Problem::cause("foo");
```

## Implicitly
Types implementing `Error` trait can be converted to `Problem` via `From` trait so that `?` will work.

```rust,skt-problem
fn foo() -> Result<String, Problem> {
    let str = String::from_utf8(vec![0, 123, 255])?;
    Ok(str)
}
assert_eq!(foo().unwrap_err().to_string(), "invalid utf-8 sequence of 1 bytes from index 2");
```

## Explicitly
Any type that implements `ToString` or `Display` can be converted to `Problem` with `.to_problem()`.

```rust,skt-problem
assert_eq!("oops".to_problem().to_string(), "oops");
```

## From Option
Often when working with C libraries actual errors may be unknown and function `Result` will have `Option<impl Error>` for their `Err` variant type.
`.to_problem()` method is implemented for `Option<E>` and will contain "<unknown error>" message for `None` variant.

```rust,skt-problem
let unknown: Option<&'static str> = None;
let known: Option<&'static str> = Some("oops");

assert_eq!(unknown.to_problem().to_string(), "<unknown error>");
assert_eq!(known.to_problem().to_string(), "oops");
```

## By mapping Result
`Result<T, E>` can be mapped into `Result<T, Problem>` with `.map_problem()` function.

```rust,skt-problem
let res: Result<(), &'static str> = Err("oops");

assert_eq!(res.map_problem().unwrap_err().to_string(), "oops");
```

## By converting Option to Result
`Option<T>` can be converted into `Result<T, Problem>` with `.ok_or_problem(message)` function.

```rust,skt-problem
let opt: Option<()> = None;

assert_eq!(opt.ok_or_problem("oops").unwrap_err().to_string(), "oops");
```

# Adding context to Problem

## Inline
Methods `.problem_while(message)` and `.problem_while_with(|| message)` can be called on any `Result` that error type can be implicitly converted to `Problem`.
The `_with` variant can be used to delay computation of error message to the moment when actual `Err` variant has occurred.

```rust,skt-problem
let res = String::from_utf8(vec![0, 123, 255]);

assert_eq!(res.problem_while("creating string").unwrap_err().to_string(), "while creating string got problem caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

## Wrapped
Functions `in_context_of(message, closure)` and `in_context_of_with(|| message, closure)` can be used to wrap block of code in closure.
This is useful when you want to add context to any error that can happen in the block of code with `?` operator.
The return type of the closure needs to be `Result<T, Problem>`.
The `_with` variant can be used to delay computation of error message to the moment when actual `Err` variant has occurred.

```rust,skt-problem
let res = in_context_of("processing string", || {
    let _s = String::from_utf8(vec![0, 123, 255])?;
    // do some processing of _s
    Ok(())
});

assert_eq!(res.unwrap_err().to_string(), "while processing string got problem caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

## Nested context
Context methods can be used multiple times to add another layer of context.

```rust,skt-problem
fn foo() -> Result<String, Problem> {
    let str = String::from_utf8(vec![0, 123, 255])?;
    Ok(str)
}

let res = in_context_of("doing stuff", || {
    let _s = foo().problem_while("running foo")?;
    // do some processing of _s
    Ok(())
});

assert_eq!(res.unwrap_err().to_string(), "while doing stuff, while running foo got problem caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

# Aborting program on Problem
`panic!(msg, problem)` macro can be used directly to abort program execution but error message printed on the screen will be formatted with `Debug` implementation.
This library provides function `format_panic_to_stderr()` to set up hook that will use `eprintln!("{}", message)` to report panics.
Function `format_panic_to_error_log()` will set up hook that will log with `error!("{}", message)` to report panics.

## Panicking on Result with Problem
Similarly to `.expect(message)`, method `.or_failed_to(message)` can be used to abort the program via `panic!()` with `Display` formatted message when called on `Err` variant of `Result` with error type implementing `Display` trait.

```rust,should_panic,skt-problem
format_panic_to_stderr();

// Prints message: Failed to convert string due to: invalid utf-8 sequence of 1 bytes from index 2
let _s = String::from_utf8(vec![0, 123, 255]).or_failed_to("convert string");
```

## Panicking on Option
Similarly to `.ok_or(error)`, method `.or_failed_to(message)` can be used to abort the program via `panic!()` with formatted message on `None` variant of `Option` type.

```rust,should_panic,skt-problem
format_panic_to_stderr();
let nothing: Option<&'static str> = None;

// Prints message: Failed to get something
let _s = nothing.or_failed_to("get something");
```

## Panicking on iterators of Result
Method `.or_failed_to(message)` can be used to abort the program via `panic!()` with formatted message on iterators with `Result` item when first `Err` is encountered otherwise unwrapping the `Ok` value.

```rust,should_panic,skt-problem
format_panic_to_stderr();

let results = vec![Ok(1u32), Ok(2u32), Err("oops")];

// Prints message: Failed to collect numbers due to: oops
let _ok: Vec<u32> = results.into_iter()
    .or_failed_to("collect numbers")
    .collect();
```