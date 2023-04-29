mod arbitrary;
mod interpreter;

use std::error::Error;
use hvm::rulebook::sanitize_rule;
use proptest::prelude::*;
use hvm::syntax::{Term, Rule};

static INSERSION_SORT: &str = "
(Sort List.nil)         = List.nil
(Sort (List.cons x xs)) = (Insert x (Sort xs))

(Insert v List.nil)         = (List.cons v List.nil)
(Insert v (List.cons x xs)) = (U60.if (> v x) (List.cons x (Insert v xs)) (List.cons v (List.cons x xs)))
";

fn vec_term<T>(values: impl IntoIterator<Item=T>) -> hvm::Term
where T: Into<hvm::Term> {
    hvm::Term::list(values.into_iter().map(|x| x.into()))
}

fn as_vec<T>(term: &hvm::Term) -> Option<Vec<T>>
where hvm::Term: TryInto<T> {
    term.as_list()?.cloned().map(|x| x.try_into().ok()).collect()
}

fn sanitize_term(term: &Term) -> Result<Term, Box<dyn Error + Sync + Send + 'static>> {
    let rule = Rule::new(Term::constructor("HVM_MAIN_CALL", []), term.clone());
    Ok(*sanitize_rule(&rule)?.rhs)
}

// returns true if the two terms are identical up to a renaming of variables
fn isomorphic(term1: &Term, term2: &Term) -> bool {
    // currently assumes both terms are sanitizeable
    // and that the names of sanitized variables only depend on order of apearance
    sanitize_term(term1).unwrap() == sanitize_term(term2).unwrap()
}

#[test]
fn insertion_sort_serial() {
    let runtime = hvm::RuntimeBuilder::default().set_thread_count(1).add_code(INSERSION_SORT).unwrap().build();
    proptest!(|(mut list in proptest::collection::vec(0u64..1048576, 0..256))| {
        let term = hvm::Term::constructor("Sort", [vec_term(list.clone())]);
        list.sort();
        let output = as_vec(&runtime.normalize_term(&term)).unwrap();
        assert_eq!(list, output);
    });
}

#[test]
fn insertion_sort_parallel() {
    let runtime = hvm::RuntimeBuilder::default().add_code(INSERSION_SORT).unwrap().build();
    proptest!(|(mut list in proptest::collection::vec(0u64..1048576, 0..256))| {
        let term = hvm::Term::constructor("Sort", [vec_term(list.clone())]);
        list.sort();
        let output: Vec<u64> = as_vec(&runtime.normalize_term(&term)).unwrap();
        assert_eq!(list, output);
    });
}

#[test]
fn compare_to_model() {
    let params = arbitrary::TermParams::default();
    let runtime = hvm::RuntimeBuilder::default().set_thread_count(1).build();
    proptest!(|(lesser_term in arbitrary::LesserTerm::arbitrary_with(params))| {
        let term: Term = lesser_term.into();
        let comp_result = std::panic::catch_unwind(|| {
            let runtime = &runtime; // shadow to prevent moving
            std::thread::scope(move |s| {
                let term_copy = term.clone();
                let hvm_handle = s.spawn(move || {
                    runtime.normalize_term(&term)
                });
                let interp_handle = s.spawn(move || {
                    interpreter::complete_dups(interpreter::normalize(term_copy))
                });
                let sec = std::time::Duration::from_secs(1);
                for _ in 0..60 {
                    if !interp_handle.is_finished() {
                        std::thread::sleep(sec);
                    } else if !hvm_handle.is_finished() {
                        // HVM should have terminated by now
                        return None;
                    } else {
                        return Some((hvm_handle.join(), interp_handle.join()));
                    }
                }
                panic!("term (possibly) does not terminate");
            });
        });

        // TODO: asset!(isomorphic(expected_result, result));
        // variable names might be different
    })
}
