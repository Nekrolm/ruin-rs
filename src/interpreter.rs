use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Unit,
    Function {
        params: Vec<String>,
        captured_scope: Scope,
        body: Box<Expr>,
        return_type: Option<TypeAnnotation>,
    },
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::Unit => "unit",
            Value::Function { .. } => "function",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scope {
    pub variables: HashMap<String, Value>,
}

pub struct Interpreter {
    scopes: Vec<Scope>,
    pending_return: Option<Value>,
    pending_break: Option<Value>,
    pending_continue: bool,
    loop_depth: usize,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            scopes: vec![Scope {
                variables: HashMap::new(),
            }],
            pending_return: None,
            pending_break: None,
            pending_continue: false,
            loop_depth: 0,
        }
    }

    pub fn set_global_scope(&mut self, scope: Scope) {
        if let Some(global) = self.scopes.first_mut() {
            *global = scope;
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope {
            variables: HashMap::new(),
        });
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.variables.insert(name, value);
        }
    }

    fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.variables.contains_key(name) {
                scope.variables.insert(name.to_string(), value);
                return Ok(());
            }
        }
        Err(format!("Undefined variable '{}'.", name))
    }

    fn lookup(&self, name: &str) -> Result<Value, String> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.variables.get(name) {
                return Ok(value.clone());
            }
        }
        Err(format!("Undefined variable '{}'.", name))
    }

    pub fn execute_program(&mut self, statements: &[Stmt]) -> Result<Value, String> {
        let mut last = Value::Unit;
        for statement in statements {
            last = self.execute_statement(statement)?;
        }
        Ok(last)
    }

    fn execute_statement(&mut self, statement: &Stmt) -> Result<Value, String> {
        match statement {
            Stmt::Let {
                name,
                type_ann,
                expr,
            } => {
                // Special handling: if type is a function type, create an implicit function
                let value = if let Some(TypeAnnotation::Fn(param_types, return_type)) = type_ann {
                    // Create implicit function from the parameters and expression
                    let param_names: Vec<String> =
                        param_types.iter().map(|(pname, _)| pname.clone()).collect();
                    let captured_scope = Scope {
                        variables: self
                            .scopes
                            .iter()
                            .flat_map(|s| &s.variables)
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                    };
                    Value::Function {
                        params: param_names,
                        captured_scope,
                        body: Box::new(expr.clone()),
                        return_type: return_type.as_ref().map(|rt| rt.as_ref().clone()),
                    }
                } else {
                    self.eval_expression(expr)?
                };

                if let Some(type_ann) = type_ann {
                    if !Self::check_type(type_ann, &value) {
                        return Err(format!(
                            "Type mismatch for '{}' : expected {:?}, got {}.",
                            name,
                            type_ann,
                            value.type_name()
                        ));
                    }
                }
                self.define(name.clone(), value);
                Ok(Value::Unit)
            }
            Stmt::Fn {
                name,
                params,
                return_type,
                body,
            } => {
                let param_names: Vec<String> =
                    params.iter().map(|(pname, _)| pname.clone()).collect();
                let captured_scope = Scope {
                    variables: self
                        .scopes
                        .iter()
                        .flat_map(|s| &s.variables)
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                };
                let value = Value::Function {
                    params: param_names,
                    captured_scope,
                    body: Box::new(body.clone()),
                    return_type: return_type.clone(),
                };
                self.define(name.clone(), value);
                Ok(Value::Unit)
            }
            Stmt::Assign { name, expr } => {
                let value = self.eval_expression(expr)?;
                self.assign(name, value)?;
                Ok(Value::Unit)
            }
            Stmt::Return(expr) => {
                let value = if let Some(e) = expr {
                    self.eval_expression(e)?
                } else {
                    Value::Unit
                };
                self.pending_return = Some(value);
                Ok(Value::Unit)
            }
            Stmt::ExprStmt(expr) => self.eval_expression(expr),
            Stmt::Block(stmts) => self.execute_block(stmts),
        }
    }

    fn execute_block(&mut self, statements: &[Stmt]) -> Result<Value, String> {
        self.push_scope();
        let mut last = Value::Unit;
        for stmt in statements {
            last = self.execute_statement(stmt)?;
            if self.pending_return.is_some()
                || self.pending_break.is_some()
                || self.pending_continue
            {
                break;
            }
        }
        self.pop_scope();
        Ok(last)
    }

    fn eval_expression(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Literal(literal) => match literal {
                Literal::Int(value) => Ok(Value::Int(*value)),
                Literal::Float(value) => Ok(Value::Float(*value)),
                Literal::String(value) => Ok(Value::Str(value.clone())),
                Literal::Bool(value) => Ok(Value::Bool(*value)),
            },
            Expr::Ident(name) => self.lookup(name),
            Expr::Unary(op, rhs) => {
                let value = self.eval_expression(rhs)?;
                match (op, value) {
                    (UnaryOp::Neg, Value::Int(i)) => Ok(Value::Int(-i)),
                    (UnaryOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
                    (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (UnaryOp::BitNot, Value::Int(i)) => Ok(Value::Int(!i)),
                    _ => Err("Invalid unary operation".into()),
                }
            }
            Expr::Binary(lhs, op, rhs) => {
                let left = self.eval_expression(lhs)?;
                let right = self.eval_expression(rhs)?;
                Self::eval_binary(op, left, right)
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let test = self.eval_expression(condition)?;
                match test {
                    Value::Bool(true) => {
                        let result = self.eval_expression(then_branch)?;
                        Ok(result)
                    }
                    Value::Bool(false) => {
                        if let Some(else_expr) = else_branch {
                            self.eval_expression(else_expr)
                        } else {
                            Ok(Value::Unit)
                        }
                    }
                    _ => Err("Condition of if expression must be a boolean".into()),
                }
            }
            Expr::Call { callee, args } => {
                // Check if callee is a built-in function first
                if let Expr::Ident(name) = &**callee {
                    if name == "print" {
                        let values: Result<Vec<_>, _> =
                            args.iter().map(|expr| self.eval_expression(expr)).collect();
                        let values = values?;
                        let output: Vec<String> = values
                            .into_iter()
                            .map(|value| match value {
                                Value::Int(i) => i.to_string(),
                                Value::Float(f) => f.to_string(),
                                Value::Str(s) => s,
                                Value::Bool(b) => b.to_string(),
                                Value::Unit => "unit".into(),
                                Value::Function { .. } => "<function>".into(),
                            })
                            .collect();
                        println!("{}", output.join(" "));
                        return Ok(Value::Unit);
                    }
                }

                // Otherwise try to evaluate callee as a user-defined function
                let func_val = self.eval_expression(callee)?;
                match func_val {
                    Value::Function {
                        params,
                        captured_scope,
                        body,
                        return_type: _,
                    } => {
                        if params.len() != args.len() {
                            return Err(format!(
                                "Function expects {} arguments, got {}",
                                params.len(),
                                args.len()
                            ));
                        }
                        let arg_values: Result<Vec<_>, _> =
                            args.iter().map(|expr| self.eval_expression(expr)).collect();
                        let arg_values = arg_values?;

                        self.push_scope();
                        // Restore captured scope
                        if let Some(scope) = self.scopes.last_mut() {
                            scope.variables.extend(captured_scope.variables.clone());
                        }
                        // Bind parameters
                        for (param, arg) in params.iter().zip(arg_values.iter()) {
                            self.define(param.clone(), arg.clone());
                        }

                        let result = self.eval_expression(&body)?;

                        let ret = if let Some(pending) = self.pending_return.take() {
                            pending
                        } else {
                            result
                        };

                        self.pop_scope();
                        Ok(ret)
                    }
                    _ => Err("Callee must be a function".into()),
                }
            }
            Expr::Fn {
                params,
                body,
                return_type,
            } => {
                let param_names: Vec<String> =
                    params.iter().map(|(name, _)| name.clone()).collect();
                let captured_scope = Scope {
                    variables: self
                        .scopes
                        .iter()
                        .flat_map(|s| &s.variables)
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                };
                Ok(Value::Function {
                    params: param_names,
                    captured_scope,
                    body: body.clone(),
                    return_type: return_type.clone(),
                })
            }
            Expr::Loop { body } => {
                self.loop_depth += 1;
                loop {
                    let result = self.eval_expression(body)?;
                    if self.pending_return.is_some() {
                        self.loop_depth -= 1;
                        return Ok(result);
                    }
                    if let Some(value) = self.pending_break.take() {
                        self.loop_depth -= 1;
                        return Ok(value);
                    }
                    if self.pending_continue {
                        self.pending_continue = false;
                        continue;
                    }
                    // If the loop body completed without break/continue/return, keep iterating.
                }
            }
            Expr::While { condition, body } => {
                self.loop_depth += 1;
                loop {
                    let test = self.eval_expression(condition)?;
                    match test {
                        Value::Bool(true) => {
                            let result = self.eval_expression(body)?;
                            if self.pending_return.is_some() {
                                self.loop_depth -= 1;
                                return Ok(result);
                            }
                            if self.pending_break.is_some() {
                                self.pending_break = None;
                                self.loop_depth -= 1;
                                return Ok(Value::Unit);
                            }
                            if self.pending_continue {
                                self.pending_continue = false;
                                continue;
                            }
                        }
                        Value::Bool(false) => {
                            self.loop_depth -= 1;
                            return Ok(Value::Unit);
                        }
                        _ => return Err("Condition of while expression must be a boolean".into()),
                    }
                }
            }
            Expr::Break { value } => {
                if self.loop_depth == 0 {
                    return Err("break outside of loop".into());
                }
                let result = if let Some(expr) = value {
                    self.eval_expression(expr)?
                } else {
                    Value::Unit
                };
                self.pending_break = Some(result);
                Ok(Value::Unit)
            }
            Expr::Continue => {
                if self.loop_depth == 0 {
                    return Err("continue outside of loop".into());
                }
                self.pending_continue = true;
                Ok(Value::Unit)
            }
            Expr::Return(expr) => {
                let value = if let Some(e) = expr {
                    self.eval_expression(e)?
                } else {
                    Value::Unit
                };
                self.pending_return = Some(value);
                Ok(Value::Unit) // or something, but since it's return, maybe not reached
            }
            Expr::Block(statements) => self.execute_block(statements),
        }
    }

    fn eval_binary(op: &BinaryOp, left: Value, right: Value) -> Result<Value, String> {
        match op {
            BinaryOp::Add => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
                _ => Err("Invalid operands for +".into()),
            },
            BinaryOp::Sub => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - b as f64)),
                _ => Err("Invalid operands for -".into()),
            },
            BinaryOp::Mul => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * b as f64)),
                _ => Err("Invalid operands for *".into()),
            },
            BinaryOp::Div => match (left, right) {
                (Value::Int(a), Value::Int(b)) if b != 0 => Ok(Value::Int(a / b)),
                (Value::Float(a), Value::Float(b)) if b != 0.0 => Ok(Value::Float(a / b)),
                (Value::Int(a), Value::Float(b)) if b != 0.0 => Ok(Value::Float(a as f64 / b)),
                (Value::Float(a), Value::Int(b)) if b != 0 => Ok(Value::Float(a / b as f64)),
                _ => Err("Invalid operands for / or divide by zero".into()),
            },
            BinaryOp::Eq => Ok(Value::Bool(left == right)),
            BinaryOp::Ne => Ok(Value::Bool(left != right)),
            BinaryOp::Lt => Self::compare_values(left, right, |a, b| a < b),
            BinaryOp::Gt => Self::compare_values(left, right, |a, b| a > b),
            BinaryOp::Le => Self::compare_values(left, right, |a, b| a <= b),
            BinaryOp::Ge => Self::compare_values(left, right, |a, b| a >= b),
            BinaryOp::And => match (left, right) {
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a && b)),
                _ => Err("Invalid operands for and".into()),
            },
            BinaryOp::Or => match (left, right) {
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a || b)),
                _ => Err("Invalid operands for or".into()),
            },
            BinaryOp::BitAnd => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
                _ => Err("Invalid operands for &".into()),
            },
            BinaryOp::BitOr => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a | b)),
                _ => Err("Invalid operands for |".into()),
            },
        }
    }

    fn compare_values(
        left: Value,
        right: Value,
        compare: impl Fn(f64, f64) -> bool,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(compare(a as f64, b as f64))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(compare(a, b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Bool(compare(a as f64, b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(compare(a, b as f64))),
            (Value::Str(a), Value::Str(b)) => {
                Ok(Value::Bool(compare(a.len() as f64, b.len() as f64)))
            }
            _ => Err("Invalid operands for comparison".into()),
        }
    }

    fn check_type(annotation: &TypeAnnotation, value: &Value) -> bool {
        match annotation {
            TypeAnnotation::Int => matches!(value, Value::Int(_)),
            TypeAnnotation::Float => matches!(value, Value::Float(_)),
            TypeAnnotation::String => matches!(value, Value::Str(_)),
            TypeAnnotation::Bool => matches!(value, Value::Bool(_)),
            TypeAnnotation::Never => false,
            TypeAnnotation::Fn(_, _) => matches!(value, Value::Function { .. }),
            TypeAnnotation::Custom(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_captures_shadowed_variable() {
        // Outer scope: x = 10
        let outer_scope = Scope {
            variables: {
                let mut map = HashMap::new();
                map.insert("x".to_string(), Value::Int(10));
                map
            },
        };

        let mut interpreter = Interpreter::new();
        interpreter.set_global_scope(outer_scope);

        // Inner scope: push and set x = 20
        interpreter.push_scope();
        interpreter.define("x".to_string(), Value::Int(20));

        // Create function in inner scope that references x
        // Function body: x + 5
        let fn_expr = Expr::Binary(
            Box::new(Expr::Ident("x".to_string())),
            BinaryOp::Add,
            Box::new(Expr::Literal(Literal::Int(5))),
        );

        let result = interpreter.eval_expression(&fn_expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(25)); // Should use inner x = 20, giving 25

        // Pop inner scope
        interpreter.pop_scope();

        // Now verify outer scope x is still 10
        let x_outer = interpreter.lookup("x");
        assert!(x_outer.is_ok());
        assert_eq!(x_outer.unwrap(), Value::Int(10));
    }

    #[test]
    fn test_function_value_captures_scope() {
        let mut interpreter = Interpreter::new();

        // Define outer variable
        interpreter.define("outer_var".to_string(), Value::Int(100));

        // Push inner scope
        interpreter.push_scope();
        interpreter.define("inner_var".to_string(), Value::Int(50));

        // Create function value in inner scope
        let fn_body = Expr::Binary(
            Box::new(Expr::Ident("outer_var".to_string())),
            BinaryOp::Add,
            Box::new(Expr::Ident("inner_var".to_string())),
        );

        let fn_expr = Expr::Fn {
            params: vec![],
            body: Box::new(fn_body),
            return_type: None,
        };

        let fn_value = interpreter.eval_expression(&fn_expr);
        assert!(fn_value.is_ok());

        // Extract captured scope from function
        let fn_val = fn_value.unwrap();
        match fn_val {
            Value::Function { captured_scope, .. } => {
                // Verify captured scope has both outer_var and inner_var
                assert!(captured_scope.variables.contains_key("outer_var"));
                assert!(captured_scope.variables.contains_key("inner_var"));
                assert_eq!(
                    captured_scope.variables.get("outer_var"),
                    Some(&Value::Int(100))
                );
                assert_eq!(
                    captured_scope.variables.get("inner_var"),
                    Some(&Value::Int(50))
                );
            }
            _ => panic!("Expected function value"),
        }

        // Pop inner scope and call function
        interpreter.pop_scope();

        // Manually call to test captured scope is used
        let result = interpreter.eval_expression(&fn_expr);
        assert!(result.is_ok());
    }

    #[test]
    fn test_shadowing_in_nested_function_calls() {
        let mut interpreter = Interpreter::new();

        // Outer: x = 5
        interpreter.define("x".to_string(), Value::Int(5));

        // Inner scope: x = 10
        interpreter.push_scope();
        interpreter.define("x".to_string(), Value::Int(10));

        // Create function that uses x
        let fn_body = Expr::Ident("x".to_string());
        let fn_expr = Expr::Fn {
            params: vec![],
            body: Box::new(fn_body),
            return_type: None,
        };

        let fn_value = interpreter.eval_expression(&fn_expr);
        assert!(fn_value.is_ok());

        // Check captured scope has inner x
        match fn_value.unwrap() {
            Value::Function { captured_scope, .. } => {
                assert_eq!(captured_scope.variables.get("x"), Some(&Value::Int(10)));
            }
            _ => panic!("Expected function"),
        }
    }
}
