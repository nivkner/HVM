mod arbitrary;

use proptest::prelude::*;

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
        let term = lesser_term.into();
        let _result = std::panic::catch_unwind(|| runtime.normalize_term(&term));
        // TODO: asset_eq!(expected_result, result);
    })
}
