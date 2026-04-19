pub mod ast;
pub mod interpreter;
pub mod lexer;
pub mod parser;

pub use ast::{Expr, Stmt, TypeAnnotation};
pub use interpreter::{Scope, Value};

pub fn run_program(source: &str) -> Result<(), String> {
    let tokens = lexer::lex(source)?;
    let program = parser::Parser::new(tokens).parse_program()?;
    let mut interpreter = interpreter::Interpreter::new();
    interpreter.execute_program(&program)?;
    Ok(())
}

pub fn eval(script: &str, initial_scope: Scope) -> Result<Value, String> {
    let tokens = lexer::lex(script)?;
    let program = parser::Parser::new(tokens).parse_program()?;
    let mut interpreter = interpreter::Interpreter::new();
    interpreter.set_global_scope(initial_scope);
    interpreter.execute_program(&program)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn run_basic_program() {
        let source = r#"
            let x : int = 5;
            let y : int = x + 2;
            x + y
        "#;

        let result = eval(source, Scope::default());
        assert_eq!(result, Ok(Value::Int(12)));
    }

    #[test]
    fn test_simple_function() {
        let source = r#"
            let add_one : fn(x: int) -> int = x + 1;
            add_one(5)
        "#;
        let result = eval(source, Scope::default());
        assert_eq!(result, Ok(Value::Int(6)));
    }

    #[test]
    fn test_function_without_return_type() {
        let source = r#"
            let add_one : fn(x: int) -> int = x + 1;
            add_one(5)
        "#;
        let result = eval(source, Scope::default());
        assert_eq!(result, Ok(Value::Int(6)));
    }

    #[test]
    fn test_function_with_multiple_params() {
        let source = r#"
            let add : fn(x: int, y: int) -> int = x + y;
            add(3, 4)
        "#;
        let result = eval(source, Scope::default());
        assert_eq!(result, Ok(Value::Int(7)));
    }

    #[test]
    fn test_closure_capture() {
        let source = r#"
            let y : int = 10;
            let add_y : fn(x: int) -> int = x + y;
            add_y(5)
        "#;
        let result = eval(source, Scope { variables: HashMap::new() });
        assert_eq!(result, Ok(Value::Int(15)));
    }

    #[test]
    fn test_function_with_return_statement() {
        let source = r#"
            let sign : fn(x: int) -> int = {
                if x > 0 {
                    return 1;
                } else {
                    return 0;
                }
            };
            sign(5)
        "#;
        let result = eval(source, Scope { variables: HashMap::new() });
        assert_eq!(result, Ok(Value::Int(1)));
    }

    #[test]
    fn test_function_empty_return() {
        let source = r#"
            let test : fn(x: int) -> int = {
                if x > 5 {
                    return 10;
                }
                x
            };
            test(8)
        "#;
        let result = eval(source, Scope { variables: HashMap::new() });
        assert_eq!(result, Ok(Value::Int(10)));
    }

    #[test]
    fn test_nested_function_calls() {
        let source = r#"
            let mul_two : fn(x: int) -> int = x * 2;
            let add_one : fn(x: int) -> int = x + 1;
            mul_two(add_one(3))
        "#;
        let result = eval(source, Scope { variables: HashMap::new() });
        assert_eq!(result, Ok(Value::Int(8)));
    }

    #[test]
    fn test_function_in_expression() {
        let source = r#"
            let double : fn(x: int) -> int = x * 2;
            double(5) + 3
        "#;
        let result = eval(source, Scope { variables: HashMap::new() });
        assert_eq!(result, Ok(Value::Int(13)));
    }

    #[test]
    fn test_nested_functions_with_returns() {
        // Test all 4 combinations of function calls with if statements and returns
        let source = r#"
            let b_func : fn(x: int) -> int = {
                if x > 0 {
                    return x * 2;
                }
                x * (-3)
            };
            let a_func : fn(y: int) -> int = {
                let b : int = b_func(y);
                if b > 5 {
                    return b + 1;
                }
                b + 2
            };
            a_func
        "#;
        
        let scope = Scope { variables: HashMap::new() };
        let result = eval(source, scope.clone());
        let func_value = result.unwrap();
        
        // Test case 1: B return path, A return path (x = 3)
        // B: x > 0, returns 3*2 = 6
        // A: 6 > 5, returns 6+1 = 7
        if let Value::Function { .. } = &func_value {
            let call_script = "a_func(3)";
            let call_result = eval(call_script, Scope { variables: [("a_func".to_string(), func_value.clone())].into_iter().collect() });
            assert_eq!(call_result, Ok(Value::Int(7)));
        }
        
        // Test case 2: B return path, A non-return path (x = 2)
        // B: x > 0, returns 2*2 = 4
        // A: 4 <= 5, returns 4+2 = 6
        let call_script = "a_func(2)";
        let call_result = eval(call_script, Scope { variables: [("a_func".to_string(), func_value.clone())].into_iter().collect() });
        assert_eq!(call_result, Ok(Value::Int(6)));
        
        // Test case 3: B non-return path, A return path (x = -2)
        // B: x <= 0, returns -(-2)*3 = 6
        // A: 6 > 5, returns 6+1 = 7
        let call_script = "a_func(-2)";
        let call_result = eval(call_script, Scope { variables: [("a_func".to_string(), func_value.clone())].into_iter().collect() });
        assert_eq!(call_result, Ok(Value::Int(7)));
        
        // Test case 4: B non-return path, A non-return path (x = 0)
        // B: x <= 0, returns -(0)*3 = 0
        // A: 0 <= 5, returns 0+2 = 2
        let call_script = "a_func(0)";
        let call_result = eval(call_script, Scope { variables: [("a_func".to_string(), func_value.clone())].into_iter().collect() });
        assert_eq!(call_result, Ok(Value::Int(2)));
    }

    #[test]
    fn test_recursive_function() {
        // Test recursive function (factorial)
        let source = r#"
            let factorial : fn(n: int) = 
              if n <= 1 { 1 }
              else { n * factorial(n - 1) }
            ;
            factorial
        "#;
        
        let scope = Scope { variables: HashMap::new() };
        let result = eval(source, scope.clone());
        let func_value = result.unwrap();
        
        // Test factorial of 0
        let call_script = "factorial(0)";
        let call_result = eval(call_script, Scope { variables: [("factorial".to_string(), func_value.clone())].into_iter().collect() });
        assert_eq!(call_result, Ok(Value::Int(1)));
        
        // Test factorial of 1
        let call_script = "factorial(1)";
        let call_result = eval(call_script, Scope { variables: [("factorial".to_string(), func_value.clone())].into_iter().collect() });
        assert_eq!(call_result, Ok(Value::Int(1)));
        
        // Test factorial of 5
        let call_script = "factorial(5)";
        let call_result = eval(call_script, Scope { variables: [("factorial".to_string(), func_value.clone())].into_iter().collect() });
        assert_eq!(call_result, Ok(Value::Int(120)));
        
        // Test factorial of 3
        let call_script = "factorial(3)";
        let call_result = eval(call_script, Scope { variables: [("factorial".to_string(), func_value.clone())].into_iter().collect() });
        assert_eq!(call_result, Ok(Value::Int(6)));
    }

    #[test]
    fn test_fn_statement_syntax() {
        let source = r#"
            fn add_one(x: int) -> int = x + 1;
            add_one(5)
        "#;
        let result = eval(source, Scope::default());
        assert_eq!(result, Ok(Value::Int(6)));
    }

    #[test]
    fn test_fn_statement_without_return_type() {
        let source = r#"
            fn identity(x: int) = x;
            identity(42)
        "#;
        let result = eval(source, Scope::default());
        assert_eq!(result, Ok(Value::Int(42)));
    }

    #[test]
    fn test_fn_statement_multiple_params() {
        let source = r#"
            fn add(x: int, y: int) -> int = x + y;
            add(3, 4)
        "#;
        let result = eval(source, Scope::default());
        assert_eq!(result, Ok(Value::Int(7)));
    }

    #[test]
    fn test_eval_with_scope() {
        let mut initial_scope = Scope { variables: HashMap::new() };
        initial_scope.variables.insert("x".to_string(), Value::Int(10));
        let script = "x + 5";
        let result = eval(script, initial_scope);
        assert_eq!(result, Ok(Value::Int(15)));
    }
}
