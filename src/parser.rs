use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, position: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn bump(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        match self.bump() {
            Some(token) if token == expected => Ok(()),
            Some(token) => Err(format!("Expected {:?}, found {:?}", expected, token)),
            None => Err(format!("Expected {:?}, found end of input", expected)),
        }
    }

    fn consume_ident(&mut self) -> Result<String, String> {
        match self.bump() {
            Some(Token::Ident(name)) => Ok(name),
            Some(token) => Err(format!("Expected identifier, found {:?}", token)),
            None => Err("Expected identifier, found end of input".into()),
        }
    }

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, String> {
        if self.check(|t| matches!(t, Token::Fn)) {
            self.bump();
            self.expect(Token::LParen)?;
            let mut params = Vec::new();
            if !self.check(|t| matches!(t, Token::RParen)) {
                loop {
                    let param_name = self.consume_ident()?;
                    self.expect(Token::Colon)?;
                    let param_type = self.parse_type_annotation()?;
                    params.push((param_name, param_type));
                    if self.check(|t| matches!(t, Token::Comma)) {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
            self.expect(Token::RParen)?;
            let return_type = if self.check(|t| matches!(t, Token::Arrow)) {
                self.bump();
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };
            Ok(TypeAnnotation::Fn(params, return_type))
        } else {
            let name = self.consume_ident()?;
            Ok(TypeAnnotation::from_name(&name))
        }
    }

    fn check(&self, predicate: impl Fn(&Token) -> bool) -> bool {
        self.peek().map_or(false, predicate)
    }

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();
        while self.peek().is_some() {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        if self.check(|t| matches!(t, Token::Let)) {
            self.bump();
            let name = self.consume_ident()?;
            self.expect(Token::Colon)?;
            let type_ann = if self.check(|t| matches!(t, Token::Assign)) {
                None
            } else {
                Some(self.parse_type_annotation()?)
            };
            self.expect(Token::Assign)?;
            let expr = self.parse_expression()?;
            self.expect(Token::Semicolon)?;
            Ok(Stmt::Let { name, type_ann, expr })
        } else if self.check(|t| matches!(t, Token::Fn)) {
            self.bump();
            let name = self.consume_ident()?;
            self.expect(Token::LParen)?;
            let mut params = Vec::new();
            if !self.check(|t| matches!(t, Token::RParen)) {
                loop {
                    let param_name = self.consume_ident()?;
                    self.expect(Token::Colon)?;
                    let param_type = self.parse_type_annotation()?;
                    params.push((param_name, param_type));
                    if self.check(|t| matches!(t, Token::Comma)) {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
            self.expect(Token::RParen)?;
            let return_type = if self.check(|t| matches!(t, Token::Arrow)) {
                self.bump();
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            self.expect(Token::Assign)?;
            let body = self.parse_expression()?;
            self.expect(Token::Semicolon)?;
            Ok(Stmt::Fn {
                name,
                params,
                return_type,
                body,
            })
        } else if self.check(|t| matches!(t, Token::Return)) {
            self.bump();
            let expr = if self.check(|t| matches!(t, Token::Semicolon)) {
                None
            } else {
                Some(self.parse_expression()?)
            };
            self.expect(Token::Semicolon)?;
            Ok(Stmt::Return(expr))
        } else if self.check(|t| matches!(t, Token::LBrace)) {
            let stmts = self.parse_block()?;
            Ok(Stmt::Block(stmts))
        } else {
            let expr = self.parse_expression()?;
            if self.check(|t| matches!(t, Token::Assign)) {
                self.bump();
                if let Expr::Ident(name) = expr {
                    let value = self.parse_expression()?;
                    self.expect(Token::Semicolon)?;
                    Ok(Stmt::Assign { name, expr: value })
                } else {
                    Err("Assignment target must be an identifier".into())
                }
            } else {
                // For expression statements, semicolon is optional if it's the last statement in a block
                if self.check(|t| matches!(t, Token::Semicolon)) {
                    self.bump();
                }
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Token::LBrace)?;
        let mut statements = Vec::new();

        while !self.check(|t| matches!(t, Token::RBrace)) {
            if self.peek().is_none() {
                return Err("Unterminated block".into());
            }
            statements.push(self.parse_statement()?);
        }

        self.expect(Token::RBrace)?;
        Ok(statements)
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_and()?;
        while self.check(|t| matches!(t, Token::Or)) {
            self.bump();
            let rhs = self.parse_and()?;
            expr = Expr::Binary(Box::new(expr), BinaryOp::Or, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bitwise_or()?;
        while self.check(|t| matches!(t, Token::And)) {
            self.bump();
            let rhs = self.parse_bitwise_or()?;
            expr = Expr::Binary(Box::new(expr), BinaryOp::And, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bitwise_and()?;
        while self.check(|t| matches!(t, Token::Pipe)) {
            self.bump();
            let rhs = self.parse_bitwise_and()?;
            expr = Expr::Binary(Box::new(expr), BinaryOp::BitOr, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_equality()?;
        while self.check(|t| matches!(t, Token::Amp)) {
            self.bump();
            let rhs = self.parse_equality()?;
            expr = Expr::Binary(Box::new(expr), BinaryOp::BitAnd, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_comparison()?;
        while self.check(|t| matches!(t, Token::Eq) || matches!(t, Token::Ne)) {
            let op = match self.bump() {
                Some(Token::Eq) => BinaryOp::Eq,
                Some(Token::Ne) => BinaryOp::Ne,
                _ => unreachable!(),
            };
            let rhs = self.parse_comparison()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_additive()?;
        while self.check(|t| matches!(t, Token::Lt) || matches!(t, Token::Gt) || matches!(t, Token::Le) || matches!(t, Token::Ge)) {
            let op = match self.bump() {
                Some(Token::Lt) => BinaryOp::Lt,
                Some(Token::Gt) => BinaryOp::Gt,
                Some(Token::Le) => BinaryOp::Le,
                Some(Token::Ge) => BinaryOp::Ge,
                _ => unreachable!(),
            };
            let rhs = self.parse_additive()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_multiplicative()?;
        while self.check(|t| matches!(t, Token::Plus) || matches!(t, Token::Minus)) {
            let op = match self.bump() {
                Some(Token::Plus) => BinaryOp::Add,
                Some(Token::Minus) => BinaryOp::Sub,
                _ => unreachable!(),
            };
            let rhs = self.parse_multiplicative()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary()?;
        while self.check(|t| matches!(t, Token::Star) || matches!(t, Token::Slash)) {
            let op = match self.bump() {
                Some(Token::Star) => BinaryOp::Mul,
                Some(Token::Slash) => BinaryOp::Div,
                _ => unreachable!(),
            };
            let rhs = self.parse_unary()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.check(|t| matches!(t, Token::Minus) || matches!(t, Token::Not) || matches!(t, Token::Tilde)) {
            let op = match self.bump() {
                Some(Token::Minus) => UnaryOp::Neg,
                Some(Token::Not) => UnaryOp::Not,
                Some(Token::Tilde) => UnaryOp::BitNot,
                _ => unreachable!(),
            };
            let rhs = self.parse_unary()?;
            Ok(Expr::Unary(op, Box::new(rhs)))
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.bump() {
            Some(Token::IntLiteral(value)) => Ok(Expr::Literal(Literal::Int(value))),
            Some(Token::FloatLiteral(value)) => Ok(Expr::Literal(Literal::Float(value))),
            Some(Token::StringLiteral(value)) => Ok(Expr::Literal(Literal::String(value))),
            Some(Token::True) => Ok(Expr::Literal(Literal::Bool(true))),
            Some(Token::False) => Ok(Expr::Literal(Literal::Bool(false))),
            Some(Token::Fn) => self.parse_fn_expression(),
            Some(Token::Loop) => self.parse_loop_expression(),
            Some(Token::While) => self.parse_while_expression(),
            Some(Token::Break) => self.parse_break_expression(),
            Some(Token::Continue) => Ok(Expr::Continue),
            Some(Token::Return) => {
                let expr = if self.check(|t| matches!(t, Token::Semicolon)) {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                Ok(Expr::Return(expr))
            }
            Some(Token::Ident(name)) => {
                if self.check(|t| matches!(t, Token::LParen)) {
                    self.bump();
                    let mut args = Vec::new();
                    if !self.check(|t| matches!(t, Token::RParen)) {
                        loop {
                            args.push(self.parse_expression()?);
                            if self.check(|t| matches!(t, Token::Comma)) {
                                self.bump();
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Call {
                        callee: Box::new(Expr::Ident(name)),
                        args,
                    })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Some(Token::LParen) => {
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Some(Token::If) => self.parse_if_expression(),
            Some(Token::LBrace) => {
                self.position -= 1;
                let statements = self.parse_block()?;
                Ok(Expr::Block(statements))
            }
            Some(token) => Err(format!("Unexpected token in expression: {:?}", token)),
            None => Err("Unexpected end of input".into()),
        }
    }

    fn parse_if_expression(&mut self) -> Result<Expr, String> {
        let condition = self.parse_expression()?;
        let then_branch = self.parse_braced_expression()?;
        let else_branch = if self.check(|t| matches!(t, Token::Else)) {
            self.bump();
            if self.check(|t| matches!(t, Token::If)) {
                let else_expr = self.parse_if_expression()?;
                Some(Box::new(else_expr))
            } else {
                Some(Box::new(self.parse_braced_expression()?))
            }
        } else {
            None
        };

        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
        })
    }

    fn parse_loop_expression(&mut self) -> Result<Expr, String> {
        let body = self.parse_braced_expression()?;
        Ok(Expr::Loop {
            body: Box::new(body),
        })
    }

    fn parse_while_expression(&mut self) -> Result<Expr, String> {
        let condition = self.parse_expression()?;
        let body = self.parse_braced_expression()?;
        Ok(Expr::While {
            condition: Box::new(condition),
            body: Box::new(body),
        })
    }

    fn parse_break_expression(&mut self) -> Result<Expr, String> {
        let value = if self.check(|t| matches!(t, Token::Semicolon) || matches!(t, Token::RBrace)) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        Ok(Expr::Break { value })
    }

    fn parse_braced_expression(&mut self) -> Result<Expr, String> {
        let stmts = self.parse_block()?;
        Ok(Expr::Block(stmts))
    }

    fn parse_fn_expression(&mut self) -> Result<Expr, String> {
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        if !self.check(|t| matches!(t, Token::RParen)) {
            loop {
                let param_name = self.consume_ident()?;
                self.expect(Token::Colon)?;
                let param_type = self.parse_type_annotation()?;
                params.push((param_name, Some(param_type)));
                if self.check(|t| matches!(t, Token::Comma)) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RParen)?;
        let return_type = if self.check(|t| matches!(t, Token::Arrow)) {
            self.bump();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let body = self.parse_expression()?;
        Ok(Expr::Fn {
            params,
            body: Box::new(body),
            return_type,
        })
    }
}
