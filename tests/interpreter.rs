#![allow(unused)]

use hvm::syntax::{Oper, Term, Rule};
use hvm::rulebook::sanitize_rule;
use std::error::Error;

pub fn normalize_term(term: &Term) -> Term {
    term.clone()
}

pub fn sanitize_term(term: &Term) -> Result<Term, Box<dyn Error + Sync + Send + 'static>> {
    let rule = Rule::new(Term::constructor("HVM_MAIN_CALL", []), term.clone());
    Ok(*sanitize_rule(&rule)?.rhs)
}
