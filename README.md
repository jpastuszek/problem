This library aims to support writing command line programs in Rust by simplifying error handling.

It introduces `Problem` type which can be used on high level APIs where error handling boils down to:
* reporting error message (e.g. log with `error!` macro),
* aborting program on error (e.g. using `panic!` macro),
* ignoring error.

# Goals
* Simplifying type signatures around error handling to one type so they compose easily
* Produce nice error messages with `Display` formatting
* Support multiple ways to add context to error message
* Support capturing information causes of the errors
* Easy conversion of existing types to `Problem` type

# Non Goals
* Providing ability to match on particular error variant for handling of error condition
* Zero allocations

# Problem type
`Problem` type is core of this library. It is basically as `String` wrapper.
When converting other errors to `Problem` the `Display` message is produced of the original error and stored in `Problem`.
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
Often when working with C libraries actual errors may not be known and will be produced as `Option<impl Error>`.

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
`Option<T>` can be mapped into `Result<T, Problem>` with `.ok_or_problem(message)` function.

```rust,skt-problem
let opt: Option<()> = None;

assert_eq!(opt.ok_or_problem("oops").unwrap_err().to_string(), "oops");
```

# Adding context to Problem

## Inline
Methods `problem_while(message)` and `problem_while_with(|| message)` can be called on any `Result` that error type can be implicitly converted to `Problem`.
The `_with` variant can be used to delay computation of message to the moment when actual `Err` variant has occurred.

```rust,skt-problem
let res = String::from_utf8(vec![0, 123, 255]);

assert_eq!(res.problem_while("creating string").unwrap_err().to_string(), "while creating string got problem caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

## Wrapped
Functions `in_context_of(message, closure)` and `in_context_of_with(|| message, closure)` can be used to wrap block of code in closure.
This is useful if you want to add context to any error that can happen in the block of code and will handled with `?` operator.
The return type of the closure needs to be `Result<T, Problem>`.
The `_with` variant can be used to delay computation of message to the moment when actual `Err` variant has occurred.

```rust,skt-problem
let res = in_context_of("processing string", || {
    let _s = String::from_utf8(vec![0, 123, 255])?;
    // do some processing of _s
    Ok(())
});

assert_eq!(res.unwrap_err().to_string(), "while processing string got problem caused by: invalid utf-8 sequence of 1 bytes from index 2");
```

# Aborting program on Problem
`panic!(msg, problem)` macro can be used directly to abort program execution but error message printed on the screen will be formatted with `Debug` implementation.
This library provides function `set_panic_hook()` to set up hook that will use `eprintln!("{}", message)` to report panics.

## Panicking on Result with Problem
Method `.or_failed_to(message)` can be used to abort the program via `panic!()` with formatted message when called on `Result` with error type implementing `Display` trait.

```rust,should_panic,skt-problem
set_panic_hook();

// Prints message: Failed to convert string due to: invalid utf-8 sequence of 1 bytes from index 2
let _s = String::from_utf8(vec![0, 123, 255]).or_failed_to("convert string");
```

## Panicking on Option
Method `.or_failed_to(message)` can be used to abort the program via `panic!()` with formatted message on `Option` type.

```rust,should_panic,skt-problem
set_panic_hook();
let nothing: Option<&'static str> = None;

// Prints message: Failed to get something
let _s = nothing.or_failed_to("get something");
```

## Panicking on iterators of Result
Method `.or_failed_to(message)` can be used to abort the program via `panic!()` with formatted message on first iterators with `Result` item when first `Err` is encountered.


```rust,should_panic,skt-problem
set_panic_hook();

let results = vec![Ok(1u32), Ok(2u32), Err("oops")];

// Prints message: Failed to collect numbers due to: oops
let _ok = results.into_iter()
    .or_failed_to("collect numbers")
    .collect::<Vec<_>>();
```