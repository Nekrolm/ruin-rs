---
description: "Ruin Programming Language Interpreter - A Rust-based interpreter for a statically-typed language with functions, arrays, and control flow."
---

# Ruin Programming Language Interpreter

This project implements a complete interpreter for the "Ruin" programming language in Rust. Ruin is a statically-typed language supporting multiple data types, first-class functions with closures, arrays, loops, and comprehensive control flow.

## Project Structure

- **`src/ast.rs`**: Abstract Syntax Tree definitions for expressions, statements, literals, operators, and type annotations
- **`src/lexer.rs`**: Lexical analyzer that tokenizes Ruin source code into a token stream
- **`src/parser.rs`**: Recursive descent parser that builds an AST from tokens with proper precedence and associativity
- **`src/interpreter.rs`**: Runtime evaluation engine with scope management (`Interpreter` struct) and value representation (`Value` enum)
- **`src/lib.rs`**: Public API with `run_program()` and `eval()` functions; exports `Scope`, `Value`, and `Interpreter`
- **`src/main.rs`**: CLI that accepts Ruin source files or stdin
- **`tests/`**: Integration tests and UI tests for comprehensive coverage
- **`example_scripts/`**: Example `.ruin` programs demonstrating language features

## Language Features (Implemented)

### Data Types
- `int` (i64): Signed integers with standard arithmetic
- `float` (f64): Floating-point numbers with implicit int→float coercion
- `string`: String literals with concatenation support via `+` operator
- `bool`: Boolean values (`true`, `false`) with logical operators
- `unit`: Unit type (void-like) returned by statements
- `array`: Fixed and variable-length arrays with element-wise type checking
- `function`: First-class functions with lexical closure capture

### Core Syntax

**Variable Declarations & Assignment**
```ruin
let x : int = 5;           // Type-annotated declaration
let y : = 10;              // Type inference from literal
let _ : int = value;       // Wildcard pattern (discards value)
x = 20;                    // Assignment to existing variable
```

**Function Definitions** (two forms)
```ruin
// Statement form: fn name(params) -> ReturnType = body;
fn add(x: int, y: int) -> int = x + y;

// Expression form: let name : fn(...) -> ReturnType = expr;
let multiply : fn(x: int, y: int) -> int = x * y;

// Closures capture enclosing scope automatically
let y : int = 10;
let add_y : fn(x: int) -> int = x + y;  // Captures y
```

**Control Flow**
```ruin
// if-then-else (expressions, not statements)
if x > 0 then 1 else -1

// Loop (infinite, terminated with break)
loop {
    if condition then break else continue;
}

// while loop
while x < 10 {
    x = x + 1;
    if x == 5 then break;
}

// Block expressions
let result : int = {
    let temp : int = calculate();
    temp * 2
};
```

**Operators**
- Arithmetic: `+`, `-`, `*`, `/` (integer and float, with mixed-type coercion)
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=` (strings compare by length)
- Logical: `and`, `or`, `not`
- Bitwise: `&`, `|`, `~` (integers only)
- Unary: `-` (negation), `not` (logical negation), `~` (bitwise NOT)

**Arrays**
```ruin
let arr : [int; _] = [1, 2, 3];      // Type-annotated array
let dynamic : = [true, false];        // Type inference
let nested : = [[1, 2], [3, 4]];      // Nested arrays
arr[0]                                // Array indexing (0-based)
arr[index + 1]                        // Expression indices
```

**Function Calls**
```ruin
print(x, y, z);                      // Built-in function
add(5, 10);                          // User-defined function
let f : fn(int) -> int = some_func;
f(42);                               // Call through variable
```

**Return Statements**
```ruin
fn sign(x: int) -> int = {
    if x > 0 then return 1;
    return -1;
};
```

### Built-in Functions
- **`print(...args)`**: Outputs space-separated values to stdout, returns unit
- **`len(array)`**: Returns array length as int, errors on non-array arguments

### Type Annotations
The language supports explicit type annotations in declarations:
- `int`, `float`, `string`, `bool`, `unit`
- `fn(param_type, ...) -> return_type` (function types)
- `[element_type; size]` or `[element_type; _]` (array types; `_` means variable-length)
- `Custom(name)` for user type extensions (reserved)

## Architecture

### Execution Pipeline

1. **Lexing** (`lexer::lex()`)
   - Input: Source string
   - Output: `Vec<Token>` with position tracking
   - Handles keywords, operators, numeric/string literals, identifiers

2. **Parsing** (`Parser::new(tokens).parse_program()`)
   - Recursive descent with operator precedence climbing
   - Main entry: `parse_program()` → statements
   - Expression hierarchy: `parse_or` → `parse_and` → `parse_bitwise_or` → ... → `parse_primary`
   - Output: `Vec<Stmt>`

3. **Interpretation** (`Interpreter::execute_program()`)
   - Holds mutable `Scope` (HashMap of variables) and scope stack for nesting
   - Processes statements sequentially
   - Evaluates expressions with side effect tracking (`pending_return`, `pending_break`, `pending_continue`)
   - Output directed through `InterpreterConfig::output` (stdout by default)

### Core Interpreter Structures

**`Value` enum** (runtime values)
```rust
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Unit,
    Array(Vec<Value>),
    Function {
        params: Vec<String>,
        captured_scope: Scope,
        body: Box<Expr>,
        return_type: Option<TypeAnnotation>,
    }
}
```

**`Scope` struct** (variable storage)
```rust
pub struct Scope {
    pub variables: HashMap<String, Value>,
}
```

**`Interpreter<'a>` struct** (mutable state during execution)
- `root_scope`: Reference to outer scope
- `local_scopes`: Stack of nested scopes for blocks/functions
- `pending_return`, `pending_break`, `pending_continue`: Control flow signals
- `loop_depth`: Tracks whether inside a loop (for break/continue validation)
- `config`: `InterpreterConfig` for output handling

### Scope and Closure Capture

Scopes form a stack (`Vec<HashMap>`). When evaluating an expression:
1. Variable lookup searches local scopes in reverse, then root scope
2. Assignment updates the first matching variable in the stack
3. Functions capture their defining scope at creation time (deep clone of all visible variables)
4. Shadowing is supported: inner scope definitions hide outer ones

## Development Guidelines

### Code Style
- Follow standard Rust conventions (snake_case for functions/variables, CamelCase for types)
- Use `Result<T, String>` for error handling; messages should be concise but descriptive
- Implement `Debug`, `Clone`, `PartialEq` on AST and value types where possible
- Comments for non-obvious logic, especially parser precedence and interpreter control flow

### Adding New Language Features

**For a new operator or syntax:**
1. Add token variant to lexer if needed (`Token::NewOp`)
2. Add AST variant: `Expr::` or `Stmt::` enum case in `src/ast.rs`
3. Add lexer recognition in `src/lexer.rs` (keyword or symbol matching)
4. Add parser rule in `src/parser.rs` respecting precedence
5. Add evaluation logic in `src/interpreter.rs` (`eval_expression()` or `execute_statement()`)
6. Add test cases to `src/interpreter.rs` or `tests/`

**For a new built-in function:**
1. Add case in `Interpreter::call_builtin()` in `src/interpreter.rs`
2. Implement argument evaluation with type checking
3. Return `Ok(Some(value))` for recognized functions, `Ok(None)` otherwise
4. Add integration tests in `tests/test_scope_persistence.rs` or new test file

### Testing

- **Unit tests**: Use `#[cfg(test)]` modules within source files, especially in `src/interpreter.rs`
  - Test scope capture, shadowing, function calls, array operations
- **Integration tests**: Use `eval()` or `run_program()` from `src/lib.rs` to test end-to-end behavior
  - Examples: `tests/test_scope_persistence.rs`, `example_scripts/`
- **UI tests**: Place `.ruin` source in `tests/ui/` with corresponding `.expected` output files
  - Test error messages and output formatting

**Commands:**
```bash
cargo test                      # Run all tests
cargo test --test '*'          # Run integration tests only
DEBUG_LEX=1 cargo run file.ruin # Debug lexer output (prints tokens)
```

### Debugging Parser Issues

1. **Check lexer output**: Enable `DEBUG_LEX=1` environment variable; interpreter prints tokens before parsing
2. **Verify token stream**: Inspect sequence against grammar rules
3. **Check operator precedence**: Ensure binary operations respect correct climbing order
4. **Add parser.rs debug output**: Insert `eprintln!()` in parser methods to trace recursion
5. **Test incrementally**: Parse simple expressions first, then complex ones

### Error Handling

**Parser errors** (during `parse_program()`)
- "Expected {token}" — unexpected syntax at position
- "Unexpected token" — unrecognized construct
- Add `?` propagation to bubble errors up

**Interpreter errors** (during `execute_program()` / `eval_expression()`)
- Type mismatches: "Type mismatch for '{var}': expected {expected}, got {actual}"
- Undefined variables: "Undefined variable '{name}'"
- Invalid operations: "Invalid operands for {op}"
- Function arity: "Function expects {N} arguments, got {M}"
- Array bounds (panics currently, could be improved)

All errors use `Result<Value, String>` with descriptive messages.

### Performance Considerations

- **Recursive evaluation**: Simple tree walking; no bytecode or optimization
- **HashMap scoping**: O(n) lookup/assignment in scope stack; suitable for small programs
- **String handling**: Uses `format!()` for concatenation and display
- **Memory**: Rust ownership handles cleanup; vectors/HashMaps are auto-freed
- **Closure capture**: Deep clones entire scope at function creation (could use Rc<RefCell<>> for efficiency)

### Output Configuration

Tests and CLI code can customize output via `InterpreterConfig`:
```rust
let config = InterpreterConfig::with_output(Box::new(my_writer));
let mut interp = Interpreter::new_with_config(&mut scope, config);
```

## Dependencies

- **`hashbrown` (0.17)**: High-performance HashMap implementation (replaces std::collections::HashMap in some builds)
- **`std::io`**: File and stdout I/O
- **`std::collections::HashMap`**: Fallback if hashbrown not used

No external crates required beyond hashbrown; minimal dependency footprint.

## File Organization Summary

| File | Responsibility |
|------|-----------------|
| `src/ast.rs` | `Expr`, `Stmt`, `Literal`, `TypeAnnotation`, operator enums; AST node definitions only |
| `src/lexer.rs` | `lex()` function; tokenization; keyword/operator recognition |
| `src/parser.rs` | `Parser` struct; `parse_program()`, precedence-climbing expression parsing |
| `src/interpreter.rs` | `Interpreter`, `Value`, `Scope` structs; execution engine; built-in functions |
| `src/lib.rs` | `run_program()`, `eval()` public API; test suite |
| `src/main.rs` | CLI entry point; file/stdin reading |

## Key Implementation Notes

### Scope Capture in Functions
Functions are created with a deep copy of the defining scope (`captured_scope`). This enables closures but means late binding is not possible. Reassigning captured variables outside the function does not affect the function's closure.

### Type Checking
Type annotations are checked at variable declaration (mismatch → error). Function return types are stored but not enforced at call sites (permissive typing for expressions).

### Array Indexing
Panics on out-of-bounds access (should be improved to return error). Supports nested indexing: `array[i][j]`.

### Control Flow
- `return` sets `pending_return` flag; breaks out of all scopes
- `break` / `continue` only valid inside loops; set flags checked in loop evaluation
- Block expressions last statement becomes the block's value

## Future Enhancements

Potential improvements (not yet implemented):
- Pattern matching (destructuring)
- Error handling with `Result` type and `?` operator
- Match expressions
- For loops and iterators
- Structs and type definitions
- Module system and imports
- Proper error recovery in parser (currently stops at first error)
- Array bounds checking instead of panics
- Type inference for variable declarations
- Optimization passes (constant folding, dead code elimination)
- Standard library expansion (more array functions, math functions, etc.)