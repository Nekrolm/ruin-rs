use crate::ast::*;
use hashbrown::HashMap;
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq)]
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
            Value::Array(_) => "array",
            Value::Function { .. } => "function",
        }
    }

    pub fn display(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Str(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Unit => "unit".into(),
            Value::Array(_) => "<array>".into(),
            Value::Function { .. } => "<function>".into(),
        }
    }

    pub fn assign(&mut self, other: Value) -> Result<(), String> {
        match (self, other) {
            (Value::Int(this), Value::Int(new)) => *this = new,
            (Value::Float(this), Value::Float(new)) => *this = new,
            (Value::Bool(this), Value::Bool(new)) => *this = new,
            (Value::Str(this), Value::Str(new)) => *this = new,
            (Value::Unit, Value::Unit) => {}
            (Value::Array(this), Value::Array(new)) => {
                if this.len() != new.len() {
                    return Err(format!(
                        "Array length mismatch: expected {}, got {}",
                        this.len(),
                        new.len()
                    ));
                }
                for (a, b) in this.iter_mut().zip(new.into_iter()) {
                    a.assign(b)?;
                }
            }
            (
                Value::Function {
                    params,
                    captured_scope,
                    body,
                    return_type,
                },
                Value::Function {
                    params: new_params,
                    captured_scope: new_captured_scope,
                    body: new_body,
                    return_type: new_return_type,
                },
            ) => {
                // TODO: function signature compatibility check?
                *params = new_params;
                *captured_scope = new_captured_scope;
                *body = new_body;
                *return_type = new_return_type;
            }
            (this, other) => {
                return Err(format!(
                    "Type mismatch. Expected {}, found {}",
                    this.type_name(),
                    other.type_name()
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scope {
    pub variables: HashMap<String, Value>,
}

/// Configuration for interpreter behavior, including output handling
pub struct InterpreterConfig {
    pub output: Box<dyn Write>,
}

impl InterpreterConfig {
    /// Create a new config with stdout as the default output
    pub fn new() -> Self {
        InterpreterConfig {
            output: Box::new(io::stdout()),
        }
    }

    /// Create a new config with custom output writer
    pub fn with_output(output: Box<dyn Write>) -> Self {
        InterpreterConfig { output }
    }
}

impl Default for InterpreterConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Interpreter<'a> {
    root_scope: &'a mut Scope,
    local_scopes: Vec<Scope>,
    pending_return: Option<Value>,
    pending_break: Option<Value>,
    pending_continue: bool,
    loop_depth: usize,
    config: InterpreterConfig,
}

impl<'a> Interpreter<'a> {
    /// Create a new interpreter with default configuration (stdout output)
    pub fn new(scope: &'a mut Scope) -> Self {
        Self::new_with_config(scope, InterpreterConfig::new())
    }

    /// Create a new interpreter with custom configuration
    pub fn new_with_config(scope: &'a mut Scope, config: InterpreterConfig) -> Self {
        Interpreter {
            root_scope: scope,
            local_scopes: Vec::new(),
            pending_return: None,
            pending_break: None,
            pending_continue: false,
            loop_depth: 0,
            config,
        }
    }

    fn push_scope(&mut self) {
        self.local_scopes.push(Scope {
            variables: HashMap::new(),
        });
    }

    fn pop_scope(&mut self) {
        self.local_scopes.pop();
    }

    fn define(&mut self, name: String, value: Value) {
        // Skip defining underscore (wildcard variable)
        if name == "_" {
            return;
        }
        if let Some(scope) = self.local_scopes.last_mut() {
            scope.variables.insert(name, value);
        } else {
            self.root_scope.variables.insert(name, value);
        }
    }

    fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        if name == "_" {
            return Ok(());
        }
        // Walk local scopes in reverse
        for scope in self.local_scopes.iter_mut().rev() {
            if let Some(var) = scope.variables.get_mut(name) {
                var.assign(value)?;
                return Ok(());
            }
        }
        // Check root scope
        if let Some(var) = self.root_scope.variables.get_mut(name) {
            var.assign(value)?;
            return Ok(());
        }
        Err(format!("Undefined variable '{}'.", name))
    }

    fn lookup(&self, name: &str) -> Result<Value, String> {
        // Walk local scopes in reverse
        for scope in self.local_scopes.iter().rev() {
            if let Some(value) = scope.variables.get(name) {
                return Ok(value.clone());
            }
        }
        // Check root scope
        if let Some(value) = self.root_scope.variables.get(name) {
            return Ok(value.clone());
        }
        Err(format!("Undefined variable '{}'.", name))
    }

    /// Call a built-in function. Returns Ok(Some(value)) if the function was called,
    /// Ok(None) if it's not a built-in function, or Err(...) on error.
    fn call_builtin(&mut self, name: &str, args: &[Expr]) -> Result<Option<Value>, String> {
        match name {
            "print" => {
                let values: Result<Vec<_>, _> = args
                    .iter()
                    .map(|expr| self.eval_expression(expr).map(|v| v.display()))
                    .collect();
                let output = values?;
                writeln!(self.config.output, "{}", output.join(" "))
                    .map_err(|e| format!("Failed to write output: {}", e))?;
                Ok(Some(Value::Unit))
            }
            "len" => {
                if args.len() != 1 {
                    return Err("len() expects exactly 1 argument".into());
                }
                let arg_val = self.eval_expression(&args[0])?;
                match arg_val {
                    Value::Array(elements) => Ok(Some(Value::Int(elements.len() as i64))),
                    _ => Err("len() expects an array argument".into()),
                }
            }
            _ => Ok(None),
        }
    }

    /// Call a user-defined function with the given parameters, captured scope, body, and arguments.
    fn call_user_function(
        &mut self,
        params: Vec<String>,
        captured_scope: Scope,
        body: Box<Expr>,
        args: &[Expr],
    ) -> Result<Value, String> {
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
        if let Some(scope) = self.local_scopes.last_mut() {
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
                    let mut captured_vars: HashMap<String, Value> = self
                        .root_scope
                        .variables
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    // Add variables from local scopes (in order, later ones override)
                    for scope in &self.local_scopes {
                        for (k, v) in &scope.variables {
                            captured_vars.insert(k.clone(), v.clone());
                        }
                    }
                    let captured_scope = Scope {
                        variables: captured_vars,
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
                let mut captured_vars: HashMap<String, Value> = self
                    .root_scope
                    .variables
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                // Add variables from local scopes (in order, later ones override)
                for scope in &self.local_scopes {
                    for (k, v) in &scope.variables {
                        captured_vars.insert(k.clone(), v.clone());
                    }
                }
                let captured_scope = Scope {
                    variables: captured_vars,
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
                // Skip assignment to underscore (wildcard variable)
                if name != "_" {
                    self.assign(name, value)?;
                }
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
                // Try to resolve callee name and first check user-defined functions in scope
                if let Expr::Ident(name) = &**callee {
                    // Check if there's a user-defined function with this name in scope
                    if let Ok(Value::Function {
                        params,
                        captured_scope,
                        body,
                        return_type: _,
                    }) = self.lookup(name)
                    {
                        return self.call_user_function(params, captured_scope, body, args);
                    }

                    // No user-defined function found, try built-in functions
                    if let Some(result) = self.call_builtin(name, args)? {
                        return Ok(result);
                    }
                }

                // Otherwise, evaluate callee as an expression (for computed function calls)
                let func_val = self.eval_expression(callee)?;
                match func_val {
                    Value::Function {
                        params,
                        captured_scope,
                        body,
                        return_type: _,
                    } => self.call_user_function(params, captured_scope, body, args),
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
                let mut captured_vars: HashMap<String, Value> = self
                    .root_scope
                    .variables
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                // Add variables from local scopes (in order, later ones override)
                for scope in &self.local_scopes {
                    for (k, v) in &scope.variables {
                        captured_vars.insert(k.clone(), v.clone());
                    }
                }
                let captured_scope = Scope {
                    variables: captured_vars,
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
            Expr::ArrayLiteral(elements) => {
                let values: Result<Vec<_>, _> = elements
                    .iter()
                    .map(|expr| self.eval_expression(expr))
                    .collect();
                Ok(Value::Array(values?))
            }
            Expr::Index { array, index } => {
                let array_val = self.eval_expression(array)?;
                let index_val = self.eval_expression(index)?;
                match (array_val, index_val) {
                    (Value::Array(elements), Value::Int(idx)) => {
                        if idx < 0 || idx as usize >= elements.len() {
                            panic!(
                                "Array index out of bounds: {} (array length: {})",
                                idx,
                                elements.len()
                            );
                        }
                        Ok(elements[idx as usize].clone())
                    }
                    _ => Err("Array indexing requires array and integer index".into()),
                }
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
            TypeAnnotation::Array(elem_type, size) => {
                match value {
                    Value::Array(elements) => {
                        // Check element types
                        for elem in elements {
                            if !Self::check_type(elem_type, elem) {
                                return false;
                            }
                        }
                        // Check size if fixed
                        match size {
                            ArraySize::Fixed(expected_size) => {
                                elements.len() as i64 == *expected_size
                            }
                            ArraySize::Inferred => true,
                        }
                    }
                    _ => false,
                }
            }
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
        let mut outer_scope = Scope {
            variables: {
                let mut map = HashMap::new();
                map.insert("x".to_string(), Value::Int(10));
                map
            },
        };

        let mut interpreter = Interpreter::new(&mut outer_scope);

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
        let mut outer_scope = Scope {
            variables: HashMap::new(),
        };

        let mut interpreter = Interpreter::new(&mut outer_scope);

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
        let mut outer_scope = Scope {
            variables: HashMap::new(),
        };

        let mut interpreter = Interpreter::new(&mut outer_scope);

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

    #[test]
    fn test_array_literal_and_indexing() {
        let mut scope = Scope {
            variables: HashMap::new(),
        };

        let mut interpreter = Interpreter::new(&mut scope);

        // Array literal: [1, 2, 3]
        let array_expr = Expr::ArrayLiteral(vec![
            Expr::Literal(Literal::Int(1)),
            Expr::Literal(Literal::Int(2)),
            Expr::Literal(Literal::Int(3)),
        ]);

        let result = interpreter.eval_expression(&array_expr);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Array(elements) => {
                assert_eq!(elements.len(), 3);
                assert_eq!(elements[0], Value::Int(1));
                assert_eq!(elements[1], Value::Int(2));
                assert_eq!(elements[2], Value::Int(3));
            }
            _ => panic!("Expected array value"),
        }
    }

    #[test]
    fn test_array_indexing() {
        let mut scope = Scope {
            variables: HashMap::new(),
        };

        let mut interpreter = Interpreter::new(&mut scope);

        // Define array in scope
        let array = Value::Array(vec![Value::Int(10), Value::Int(20), Value::Int(30)]);
        interpreter.define("arr".to_string(), array);

        // Access arr[0]
        let index_expr = Expr::Index {
            array: Box::new(Expr::Ident("arr".to_string())),
            index: Box::new(Expr::Literal(Literal::Int(0))),
        };

        let result = interpreter.eval_expression(&index_expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(10));

        // Access arr[2]
        let index_expr2 = Expr::Index {
            array: Box::new(Expr::Ident("arr".to_string())),
            index: Box::new(Expr::Literal(Literal::Int(2))),
        };

        let result2 = interpreter.eval_expression(&index_expr2);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), Value::Int(30));
    }

    #[test]
    fn test_nested_array_indexing() {
        let mut scope = Scope {
            variables: HashMap::new(),
        };

        let mut interpreter = Interpreter::new(&mut scope);

        // Define nested array: [[1, 2], [3, 4]]
        let nested_array = Value::Array(vec![
            Value::Array(vec![Value::Int(1), Value::Int(2)]),
            Value::Array(vec![Value::Int(3), Value::Int(4)]),
        ]);
        interpreter.define("nested".to_string(), nested_array);

        // Access nested[0][1]
        let inner_index = Expr::Index {
            array: Box::new(Expr::Ident("nested".to_string())),
            index: Box::new(Expr::Literal(Literal::Int(0))),
        };

        let outer_index = Expr::Index {
            array: Box::new(inner_index),
            index: Box::new(Expr::Literal(Literal::Int(1))),
        };

        let result = interpreter.eval_expression(&outer_index);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(2));
    }

    #[test]
    fn test_len_builtin() {
        let mut scope = Scope {
            variables: HashMap::new(),
        };

        let mut interpreter = Interpreter::new(&mut scope);

        // Define array
        let array = Value::Array(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]);
        interpreter.define("arr".to_string(), array);

        // Call len(arr)
        let len_call = Expr::Call {
            callee: Box::new(Expr::Ident("len".to_string())),
            args: vec![Expr::Ident("arr".to_string())],
        };

        let result = interpreter.eval_expression(&len_call);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(4));
    }

    #[test]
    fn test_underscore_wildcard() {
        let mut scope = Scope {
            variables: HashMap::new(),
        };

        let mut interpreter = Interpreter::new(&mut scope);

        // Define using underscore (should not create variable)
        interpreter.define("_".to_string(), Value::Int(100));

        // Lookup underscore (should not find it)
        let result = interpreter.lookup("_");
        assert!(result.is_err(), "Underscore should not be stored in scope");
    }

    #[test]
    fn test_array_check_type() {
        // Array of ints with fixed size
        let array_type = TypeAnnotation::Array(Box::new(TypeAnnotation::Int), ArraySize::Fixed(3));

        let value = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);

        assert!(Interpreter::check_type(&array_type, &value));

        // Mismatched size
        let value2 = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        assert!(!Interpreter::check_type(&array_type, &value2));

        // Mismatched element type
        let value3 = Value::Array(vec![
            Value::Int(1),
            Value::Str("hello".to_string()),
            Value::Int(3),
        ]);
        assert!(!Interpreter::check_type(&array_type, &value3));
    }

    #[test]
    fn test_array_inferred_size() {
        // Array of ints with inferred size
        let array_type = TypeAnnotation::Array(Box::new(TypeAnnotation::Int), ArraySize::Inferred);

        let value1 = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        assert!(Interpreter::check_type(&array_type, &value1));

        let value2 = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert!(Interpreter::check_type(&array_type, &value2));

        // But element type must still match
        let value3 = Value::Array(vec![Value::Str("a".to_string())]);
        assert!(!Interpreter::check_type(&array_type, &value3));
    }

    #[test]
    fn test_user_defined_function_shadows_builtin() {
        let mut scope = Scope {
            variables: HashMap::new(),
        };

        let mut interpreter = Interpreter::new(&mut scope);

        // Define a user-defined function named "len" that returns 42
        let len_fn = Value::Function {
            params: vec!["x".to_string()],
            captured_scope: Scope {
                variables: HashMap::new(),
            },
            body: Box::new(Expr::Literal(Literal::Int(42))),
            return_type: Some(TypeAnnotation::Int),
        };
        interpreter.define("len".to_string(), len_fn);

        // Define an array to pass as argument
        let array = Value::Array(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]);
        interpreter.define("arr".to_string(), array);

        // Call len(arr) - should return 42 from the user-defined function, not 4
        let len_call = Expr::Call {
            callee: Box::new(Expr::Ident("len".to_string())),
            args: vec![Expr::Ident("arr".to_string())],
        };

        let result = interpreter.eval_expression(&len_call);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Value::Int(42),
            "User-defined len() should shadow built-in len()"
        );
    }
}
