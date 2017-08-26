#![deny(warnings)]

use lox_scanner::TT;
use lox_parser::{Expr, Stmt};
use lox_environment::Environment;
use std::fmt;


#[derive(Clone,Debug,PartialEq)]
pub enum V {
    Nil,
    Num(f64),
    Bool(bool),
    Str(String),
}

impl V {
    fn is_truthy(&self) -> bool {
        match self {
            &V::Nil => false,
            &V::Bool(ref b) => *b,
            _ => true
        }
    }

    fn num(&self) -> Result<f64, String> {
        match self {
            &V::Num(ref n) => Ok(*n),
            o => Err(format!("expected V::Num, found {:?}", o))
        }
    }
}

impl fmt::Display for V {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &V::Nil => write!(f, "nil"),
            &V::Bool(ref b) => write!(f, "{}", b),
            &V::Num(ref n) => write!(f, "{}", n),
            &V::Str(ref s) => write!(f, "\"{}\"", s),
        }
    }
}

type EvalResult = Result<V, String>;

pub struct LoxInterpreter {
    env: Environment,
    errors: bool,
}

impl LoxInterpreter {
    pub fn new() -> Self {
        LoxInterpreter{env: Environment::new(), errors: false}
    }

    fn eval(&mut self, expr: &Expr) -> EvalResult {
        match expr {
            &Expr::Nil => Ok(V::Nil),
            &Expr::Num(n) => Ok(V::Num(n)),
            &Expr::Str(ref s) => Ok(V::Str(s.to_string())),
            &Expr::Bool(ref b) => Ok(V::Bool(*b)),
            &Expr::Grouping(ref expr) => self.eval(&*expr),
            &Expr::Unary(ref op, ref expr) => {
                let expr = self.eval(expr)?;
                match op.token {
                    TT::MINUS => Ok(V::Num(-expr.num()?)),
                    TT::BANG => Ok(V::Bool(!expr.is_truthy())),
                    _ => unreachable!("LoxIntepreter: bad Unary op {:?}", op)
                }
            },
            &Expr::Binary(ref lhs, ref op, ref rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                match op.token {
                    TT::SLASH => Ok(V::Num(lhs.num()? / rhs.num()?)),
                    TT::STAR => Ok(V::Num(lhs.num()? * rhs.num()?)),
                    TT::MINUS => Ok(V::Num(lhs.num()? - rhs.num()?)),
                    TT::PLUS => match (&lhs, &rhs) {
                        (&V::Num(ref l), &V::Num(ref r)) => Ok(V::Num(l + r)),
                        (&V::Str(ref l), &V::Str(ref r)) =>
                            Ok(V::Str(format!("{}{}", l, r))),
                        (&V::Str(ref l), ref other) =>
                            Ok(V::Str(format!("{}{}", l, other))),
                        (ref other, &V::Str(ref r)) =>
                            Ok(V::Str(format!("{}{}", other, r))),
                        _ => Err(format!("can't {:?} + {:?}", lhs, rhs))
                    },
                    TT::GT => Ok(V::Bool(lhs.num()? > rhs.num()?)),
                    TT::GE => Ok(V::Bool(lhs.num()? >= rhs.num()?)),
                    TT::LT => Ok(V::Bool(lhs.num()? < rhs.num()?)),
                    TT::LE => Ok(V::Bool(lhs.num()? <= rhs.num()?)),
                    TT::EQ => Ok(V::Bool(lhs == rhs)),
                    TT::NE => Ok(V::Bool(lhs != rhs)),
                    _ => unreachable!("LoxIntepreter: bad Binary op {:?}", op)
                }
            },
            &Expr::Var(ref var) => self.env.get(var),
            &Expr::Assign(ref var, ref expr) => {
                let value = self.eval(expr)?;
                self.env.assign(var.clone(), value)
            }
        }
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>) -> Option<String> {
        for stmt in statements {
            match stmt {
                &Stmt::Expr(ref expr) => if let Err(err) = self.eval(expr) {
                    self.errors = true;
                    return Some(err);
                },
                &Stmt::Print(ref expr) => match self.eval(expr) {
                    Ok(value) => println!("{}", value),
                    Err(err) => { self.errors = true; return Some(err) }
                },
                &Stmt::Var(ref name, ref init) => {
                    let value = match self.eval(init) {
                        Err(err) => { self.errors = true; return Some(err) },
                        Ok(value) => value
                    };
                    self.env.define(name.to_string(), value);
                }
            }
        }
        None
    }
}