#![allow(unused)]

use hvm::syntax::{Oper, Term, Rule};
use hvm::rulebook::sanitize_rule;
use std::error::Error;

static ERA: &str = "*";

// reduces term to Normal Form
pub fn normalize_term(term: &Term) -> Term {
    let mut term_copy = term.clone();
    reduce_weak(term_copy)
}

// reduces term to Weak Head Normal Form
fn reduce_weak(mut term: Term) -> Term {
    use Term::*;
    loop {
        match term {
            U6O { ref numb } => return term,
            F6O { ref numb } => return term,
            Var { ref name } => return term,
            Lam { ref name, ref body } => return term,
            Ctr { name, args } => unreachable!("constructors are not generated by proptest"),
            Let { name, expr, body } => unreachable!("let expressions are not generated by proptest"),
            Sup { ref val0, ref val1 } => return term,
            App { func, argm } => {
                // reduce func so that we know how to reduce the application
                match reduce_weak(*func) {
                    // (λx(body) a)
                    // ------------ APP-LAM
                    // x <- a
                    // body
                    Lam {name, mut body} => {
                        era_aware_substitute(&name, *argm, &mut body);
                        term = *body;
                    }
                    // ({a b} c)
                    // --------------- APP-SUP
                    // dup x0 x1 = c
                    // {(a x0) (b x1)}
                    Sup { val0, val1 } => {
                        let body = Box::new(Sup {
                            val0: Box::new(Term::application( *val0, Term::variable("x0"))),
                            val1: Box::new(Term::application( *val1, Term::variable("x1"))),
                        });
                        term = Dup { nam0: String::from("x0"), nam1: String::from("x1"), expr: argm, body};
                    }
                    func => {
                        // allow applications with no associated rules, to match HVM behavior
                        return App { func: Box::new(func), argm };
                    },
                }
            },
            Dup { nam0, nam1, expr, mut body } => {
                // reduce expr so that we know what we are duplicating
                match reduce_weak(*expr) {
                    // dup a b = λx(body)
                    // ------------------ DUP-LAM
                    // a <- λx0(b0)
                    // b <- λx1(b1)
                    // x <- {x0 x1}
                    // dup b0 b1 = body
                    Lam {name, body: mut inner_body} => {
                        let mut sup = Sup {
                            val0: Box::new(Term::variable("x0")),
                            val1: Box::new(Term::variable("x1")),
                        };
                        era_aware_substitute(&name, sup, &mut inner_body);
                        era_aware_substitute(&nam0, Term::lambda("x0", Term::variable("b0")), &mut body);
                        era_aware_substitute(&nam1, Term::lambda("x1", Term::variable("b1")), &mut body);
                        term = Dup { nam0: String::from("b0"), nam1: String::from("b1"), expr: inner_body, body: body};
                    },
                    // dup a b = {r s}
                    // --------------- DUP-SUP
                    // a <- r
                    // b <- s
                    Sup {val0, val1} => {
                        era_aware_substitute(&nam0, *val0, &mut body);
                        era_aware_substitute(&nam1, *val1, &mut body);
                        term = *body;
                    },
                    // dup a b = N
                    // --------------- DUP-{U60, F60}
                    // a <- N
                    // b <- N
                    expr @ U6O { numb: _ } | expr @ F6O { numb: _ } => {
                        era_aware_substitute(&nam0, expr.clone(), &mut body);
                        era_aware_substitute(&nam1, expr, &mut body);
                        term = *body;
                    },
                    // do not duplicate other expressions to maintain linearity
                    expr => return Dup { nam0, nam1, expr: Box::new(expr), body },
                }
            }
            _ => todo!("not there yet"),
        }
    }
}

// behaves like substitute, but returns early when the target name is ERA
fn era_aware_substitute(target: &str, expression: Term, body: &mut Term) {
    if target != ERA {
        substitute(target, expression, body)
    }
}

// substitute the variable matching the given target name, with the expression in body
fn substitute(target: &str, expression: Term, body: &mut Term) {
    use Term::*;
    match body {
        U6O { numb } => {},
        F6O { numb } => {},
        Var { name } if name == target => *body = expression,
        Var { name } => {},
        // only substitute if the variable isn't shadowed
        Lam { name, body } if name != target => substitute(target, expression, body),
        Lam { name, body } => {},
        Ctr { name, args } => unreachable!("constructors are not generated by proptest"),
        Let { name, expr, body } => unreachable!("let expressions are not generated by proptest"),
        Sup { val0, val1 } => {
            substitute(target, expression.clone(), val0);
            substitute(target, expression, val1);
        },
        App { func, argm } => {
            substitute(target, expression.clone(), func);
            substitute(target, expression, argm);
        },
        // only substitute in body if the variable isn't shadowed
        Dup { nam0, nam1, expr, body } if nam0 != target && nam1 != target => {
            substitute(target, expression.clone(), expr);
            substitute(target, expression, body);
        },
        Dup { nam0, nam1, expr, body } => substitute(target, expression, expr),
        Op2 { oper, val0, val1 } => {
            substitute(target, expression.clone(), val0);
            substitute(target, expression, val1);
        },
    }
}

pub fn sanitize_term(term: &Term) -> Result<Term, Box<dyn Error + Sync + Send + 'static>> {
    let rule = Rule::new(Term::constructor("HVM_MAIN_CALL", []), term.clone());
    Ok(*sanitize_rule(&rule)?.rhs)
}
