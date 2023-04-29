#![allow(unused)]
use hvm::syntax::{Oper, Term};

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
                        substitute(&name, *argm, &mut body);
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
                        // no matching rules for this application
                        return Term::application(func, *argm);
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
                        substitute(&name, sup, &mut inner_body);
                        substitute(&nam0, Term::lambda("x0", Term::variable("b0")), &mut body);
                        substitute(&nam1, Term::lambda("x1", Term::variable("b1")), &mut body);
                        term = Dup { nam0: String::from("b0"), nam1: String::from("b1"), expr: inner_body, body: body};
                    },
                    // dup a b = {r s}
                    // --------------- DUP-SUP
                    // a <- r
                    // b <- s
                    Sup {val0, val1} => {
                        substitute(&nam0, *val0, &mut body);
                        substitute(&nam1, *val1, &mut body);
                        term = *body;
                    },
                    // dup a b = N
                    // --------------- DUP-{U60, F60}
                    // a <- N
                    // b <- N
                    expr @ U6O { numb: _ } | expr @ F6O { numb: _ } => {
                        substitute(&nam0, expr.clone(), &mut body);
                        substitute(&nam1, expr, &mut body);
                        term = *body;
                    },
                    // no matching rules for this duplication
                    expr => return Dup { nam0, nam1, expr: Box::new(expr), body },
                }
            },
            Op2 { oper, val0, val1 } => {
                use Oper::*;
                // reduce the values so that we know how to reduce the operation
                match (reduce_weak(*val0), reduce_weak(*val1)) {
                    // (+ {a0 a1} b)
                    // --------------------- OP2-SUP-A
                    // dup b0 b1 = b
                    // {(+ a0 b0) (+ a1 b1)}
                    (Sup { val0: val0_inner, val1: val1_inner }, val1) => {
                        let sup = Sup {
                            val0: Box::new(Term::binary_operator(oper, *val0_inner, Term::variable("b0"))),
                            val1: Box::new(Term::binary_operator(oper, *val1_inner, Term::variable("b1")))
                        };
                        term = Dup { nam0: String::from("b0"), nam1: String::from("b1"), expr: Box::new(val1), body: Box::new(sup) };
                    },
                    // (+ a {b0 b1})
                    // --------------------- OP2-SUP-B
                    // dup a0 a1 = a
                    // {(+ a0 b0) (+ a1 b1)}
                    (val0, Sup { val0: val0_inner, val1: val1_inner }) => {
                        let sup = Sup {
                            val0: Box::new(Term::binary_operator(oper, Term::variable("a0"), *val0_inner)),
                            val1: Box::new(Term::binary_operator(oper, Term::variable("a1"), *val1_inner))
                        };
                        term = Dup { nam0: String::from("a0"), nam1: String::from("a1"), expr: Box::new(val0), body: Box::new(sup) };
                    },
                    // (+ N M)
                    // --------------------- OP2-U60
                    // N + M
                    ( U6O { numb: n0 }, U6O { numb: n1 } ) => {
                        term = match oper {
                            Add => Term::integer(n0 + n1),
                            Sub => Term::integer(n0 - n1),
                            Mul => Term::integer(n0 * n1),
                            Div => Term::integer(n0 / n1),
                            Mod => Term::integer(n0 % n1),
                            And => Term::integer(n0 & n1),
                            Or  => Term::integer(n0 | n1),
                            Xor => Term::integer(n0 ^ n1),
                            Shl => Term::integer(n0 << n1),
                            Shr => Term::integer(n0 >> n1),
                            Lte => Term::integer((n0 <= n1).into()),
                            Ltn => Term::integer((n0 < n1).into()),
                            Eql => Term::integer((n0 == n1).into()),
                            Gte => Term::integer((n0 >= n1).into()),
                            Gtn => Term::integer((n0 > n1).into()),
                            Neq => Term::integer((n0 != n1).into()),
                        };
                    },
                    ( val0 @ F6O { numb: _ }, val1 @ F6O { numb: _ } ) => {
                        let n0: f64 = val0.try_into().unwrap();
                        let n1: f64 = val1.try_into().unwrap();
                        term = match oper {
                            Add => Term::float(n0 + n1),
                            Sub => Term::float(n0 - n1),
                            Mul => Term::float(n0 * n1),
                            Div => Term::float(n0 / n1),
                            Mod => Term::float(n0 % n1),
                            Lte => Term::float((n0 <= n1).into()),
                            Ltn => Term::float((n0 < n1).into()),
                            Eql => Term::float((n0 == n1).into()),
                            Gte => Term::float((n0 >= n1).into()),
                            Gtn => Term::float((n0 > n1).into()),
                            Neq => Term::float((n0 != n1).into()),
                            // the following operations would be better served by a builtin func,
                            // but this is to maintain parity with HVM.
                            And => Term::float(f64::cos(n0) + f64::sin(n1)),
                            Or  => Term::float(f64::atan2(n0, n1)),
                            Xor => Term::float(n0.ceil() + n0.floor()),
                            Shl => Term::float(n1.powf(n0)),
                            Shr => Term::float(n0.log(n1)),
                        };
                    },
                    // no matching rules for this operation
                    (val0, val1) => return Term::binary_operator(oper, val0, val1),
                }
            }
        }
    }
}

// substitute the variable matching the given target name, with the expression in body,
// unless the target name is ERA
fn substitute(target: &str, expression: Term, body: &mut Term) {
    if target != ERA {
        inner_substitute(target, expression, body)
    }
}

// substitute the variable matching the given target name, with the expression in body
fn inner_substitute(target: &str, expression: Term, body: &mut Term) {
    use Term::*;
    match body {
        U6O { numb } => {},
        F6O { numb } => {},
        Var { name } if name == target => *body = expression,
        Var { name } => {},
        // only substitute if the variable isn't shadowed
        Lam { name, body } if name != target => inner_substitute(target, expression, body),
        Lam { name, body } => {},
        Ctr { name, args } => unreachable!("constructors are not generated by proptest"),
        Let { name, expr, body } => unreachable!("let expressions are not generated by proptest"),
        Sup { val0, val1 } => {
            inner_substitute(target, expression.clone(), val0);
            inner_substitute(target, expression, val1);
        },
        App { func, argm } => {
            inner_substitute(target, expression.clone(), func);
            inner_substitute(target, expression, argm);
        },
        // only substitute in body if the variable isn't shadowed
        Dup { nam0, nam1, expr, body } if nam0 != target && nam1 != target => {
            inner_substitute(target, expression.clone(), expr);
            inner_substitute(target, expression, body);
        },
        Dup { nam0, nam1, expr, body } => inner_substitute(target, expression, expr),
        Op2 { oper, val0, val1 } => {
            inner_substitute(target, expression.clone(), val0);
            inner_substitute(target, expression, val1);
        },
    }
}
