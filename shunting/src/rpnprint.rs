use lexers::{MathToken, TokenAssoc};
use parser::RPNExpr;
use std::fmt;

#[derive(Debug, Clone)]
enum AST<'a> {
    Leaf(&'a MathToken),
    Node(&'a MathToken, Vec<AST<'a>>),
}

impl RPNExpr {
    fn build_ast<'a>(&'a self) -> AST<'a> {
        let mut ops = Vec::new();
        for token in self.iter() {
            match *token {
                MathToken::Number(_) |
                MathToken::Variable(_) => ops.push(AST::Leaf(token)),
                MathToken::Function(_, arity) |
                MathToken::Op(_, arity) => {
                    let n = ops.len() - arity;
                    let node = AST::Node(token, ops.iter().skip(n).cloned().collect());
                    ops.truncate(n);
                    ops.push(node);
                },
                _ => unreachable!()
            }
        }
        ops.pop().unwrap()
    }
}

impl fmt::Display for RPNExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        fn printer(root: &AST) -> (String, (usize, TokenAssoc)) {
            match root {
                &AST::Leaf(ref token) => {
                    match *token {
                        &MathToken::Number(ref x)   => (format!("{}", x), token.precedence()),
                        &MathToken::Variable(ref x) => (format!("{}", x), token.precedence()),
                        _ => unreachable!()
                    }
                },
                &AST::Node(ref token, ref args) => {
                    match *token {
                        &MathToken::Op(ref op, arity) if arity == 1 => {
                            let subtree = printer(&args[0]);
                            let (prec, assoc) = token.precedence();
                            // TODO: distinguish perfix/postfix operators
                            if prec > (subtree.1).0 {
                                (format!("{}({})", op, subtree.0), (prec, assoc))
                            } else {
                                (format!("{}{}", op, subtree.0), (prec, assoc))
                            }
                        },
                        &MathToken::Op(ref op, arity) if arity == 2 => {
                            let (lhs, rhs) = (printer(&args[0]), printer(&args[1]));
                            let (prec, assoc) = token.precedence();

                            let lh = if prec > (lhs.1).0 ||
                                        (prec == (lhs.1).0 && assoc != TokenAssoc::Left) {
                                format!("({})", lhs.0)
                            } else {
                                format!("{}", lhs.0)
                            };
                            let rh = if prec > (rhs.1).0 ||
                                        (prec == (rhs.1).0 && assoc != TokenAssoc::Right) {
                                format!("({})", rhs.0)
                            } else {
                                format!("{}", rhs.0)
                            };
                            // NOTE: '2+(3+4)' will show parens to indicate that user
                            // explicitly put them there
                            (format!("{} {} {}", lh, op, rh), (prec, assoc))

                        },
                        &MathToken::Function(ref func, _) => {
                            let expr = args.iter()
                                .map(|leaf| printer(&leaf).0)
                                .collect::<Vec<String>>()
                                .join(", ");
                            (format!("{}({})", func, expr), token.precedence())
                        },
                        _ => unreachable!()
                    }
                }
            }
        }

        write!(f, "{}", printer(&self.build_ast()).0)
    }
}
