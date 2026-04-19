---
description: "Ruin Programming Language Interpreter - A Rust-based interpreter for a simple programming language with variables, expressions, control flow, and built-in functions."
---

# Ruin Programming Language Interpreter

This project implements a complete interpreter for the "Ruin" programming language in Rust. Ruin is a simple, statically-typed language with support for basic data types, arithmetic operations, control flow, and function calls.

## Project Structure

- **`src/ast.rs`**: Abstract Syntax Tree definitions for expressions, statements, and type annotations
- **`src/lexer.rs`**: Lexical analyzer that tokenizes source code
- **`src/parser.rs`**: Recursive descent parser that builds AST from tokens
- **`src/interpreter.rs`**: Runtime execution engine with scoping and value evaluation
- **`src/lib.rs`**: Main library interface with `run_program()` function
- **`src/main.rs`**: Command-line interface that reads source files or stdin

## Language Features

### Data Types
- `int` (i64): Integer numbers
- `float` (f64): Floating-point numbers
- `string`: String literals
- `bool`: Boolean values
- `unit`: Unit type for statements

### Syntax
- Variable declarations: `let x : int = 5;`
- Function definitions: `fn func_name(param: type, ...) -> return_type = expression;`
- Assignments: `x = 10;`
- Arithmetic: `+`, `-`, `*`, `/`
- Comparisons: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical: `and`, `or`, `not`
- Bitwise: `&`, `|`, `~`
- Control flow: `if condition then expr else expr`
- Function calls: `print(arg1, arg2, ...)`
- Blocks: `{ stmt1; stmt2; }`

### Built-in Functions
- `print(...)`: Prints arguments to stdout separated by spaces

## Development Guidelines

### Code Style
- Follow standard Rust conventions
- Use `Result<T, String>` for error handling with descriptive error messages
- Implement `Debug`, `Clone`, `PartialEq` traits on AST nodes where appropriate
- Use meaningful variable names and add comments for complex logic

### Testing
- Add unit tests in `#[cfg(test)]` modules
- Test parsing, interpretation, and error cases
- Use `assert!(run_program(source).is_ok())` for integration tests
- Run tests with `cargo test`

### Building and Running
- Build: `cargo build`
- Run tests: `cargo test`
- Run interpreter: `cargo run [source_file]` or `cargo run < source.ruin`
- Debug lexer: `DEBUG_LEX=1 cargo run source.ruin`

### Common Tasks

**Adding a new language feature:**
1. Extend AST in `ast.rs` (add new enum variants)
2. Update lexer in `lexer.rs` (add new tokens)
3. Update parser in `parser.rs` (add parsing logic)
4. Update interpreter in `interpreter.rs` (add evaluation logic)
5. Add tests and ensure existing tests pass

**Adding a built-in function:**
1. Add case in `Interpreter::evaluate_call()` method
2. Handle argument evaluation and type checking
3. Add tests for the new function

**Fixing parser errors:**
1. Check lexer output first with `DEBUG_LEX=1`
2. Verify token sequence matches expected grammar
3. Check precedence and associativity in binary operations
4. Add proper error messages for invalid syntax

### Error Handling
- Parser errors: Invalid syntax, unexpected tokens
- Interpreter errors: Type mismatches, undefined variables, invalid operations
- Runtime errors: Division by zero, invalid function calls

### Performance Considerations
- Simple recursive evaluation (no optimization yet)
- HashMap-based scoping (could be optimized for deeper nesting)
- String concatenation uses format! macro
- No garbage collection (Rust handles memory management)

## File Organization

Keep related functionality in separate modules:
- AST changes → `ast.rs`
- Tokenization → `lexer.rs`
- Grammar parsing → `parser.rs`
- Execution logic → `interpreter.rs`
- CLI interface → `main.rs`
- Public API → `lib.rs`

## Dependencies

Currently minimal dependencies - only standard library. Uses:
- `std::collections::HashMap` for variable scoping
- `std::io` for file/stdin reading
- Basic error handling with `Result` and `String`

## Future Enhancements

Potential areas for extension:
- More data types (arrays, objects)
- User-defined functions
- Loops and iteration
- Modules and imports
- Type inference
- Optimization passes
- Standard library expansion</content>
<parameter name="filePath">/home/dmis/WORKSPACE/ruin-rs/AGENTS.md