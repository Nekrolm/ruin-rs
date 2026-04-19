#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeAnnotation {
    Int,
    Float,
    String,
    Bool,
    Fn(Vec<(String, TypeAnnotation)>, Option<Box<TypeAnnotation>>),
    Custom(String),
}

impl TypeAnnotation {
    pub fn from_name(name: &str) -> Self {
        match name {
            "i32" => TypeAnnotation::Int,
            "f64" => TypeAnnotation::Float,
            "bool" => TypeAnnotation::Bool,
            "String" => TypeAnnotation::String,
            other => TypeAnnotation::Custom(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Ident(String),
    Unary(UnaryOp, Box<Expr>),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Fn {
        params: Vec<(String, Option<TypeAnnotation>)>,
        body: Box<Expr>,
        return_type: Option<TypeAnnotation>,
    },
    Return(Option<Box<Expr>>),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        type_ann: Option<TypeAnnotation>,
        expr: Expr,
    },
    Fn {
        name: String,
        params: Vec<(String, TypeAnnotation)>,
        return_type: Option<TypeAnnotation>,
        body: Expr,
    },
    Assign {
        name: String,
        expr: Expr,
    },
    Return(Option<Expr>),
    ExprStmt(Expr),
    Block(Vec<Stmt>),
}
