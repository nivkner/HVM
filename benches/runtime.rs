use criterion::{black_box, criterion_group, criterion_main, Criterion};

macro_rules! benchmark_example {
    // This macro takes a name, a path to an example HVM file and a term,
    // and creates a benchmark which interprets the given HVM file,
    // and evaluates the given term with it.
    ($func_name:ident, $path:expr, $term:expr) => {
        fn $func_name(c: &mut Criterion) {
            run_example(c, stringify!($func_name), include_str!($path), $term);
        }
    };
}

fn run_example(c: &mut Criterion, name: &str, code: &str, term: &str) {
  let thread_count = hvm::default_heap_tids();
  let file = hvm::syntax::read_file(code).unwrap();

  // Parses and reads the input file
  let file = hvm::syntax::read_file(&format!("{file}\nHVM_MAIN_CALL = {term}")).unwrap();
  // Converts the file to a Rulebook
  let book = hvm::rulebook::gen_rulebook(&file);

  // Creates the runtime program
  let mut prog = hvm::Program::new();

  // Adds the interpreted functions (from the Rulebook)
  prog.add_book(&book);

  // Creates the runtime heap
  let heap = hvm::new_heap(hvm::default_heap_size(), thread_count);
  let tids = hvm::new_tids(thread_count);

  hvm::link(&heap, 0, hvm::Fun(*book.name_to_id.get("HVM_MAIN_CALL").unwrap(), 0));
  let host = 0;

  c.bench_function(&format!("{name}, serial"), |b| {
    b.iter(|| {
        hvm::normalize(&heap, &prog, &tids[..1], black_box(host), false);
    })
  });

  c.bench_function(&format!("{name}, parallel"), |b| {
    b.iter(|| {
        hvm::normalize(&heap, &prog, &tids, black_box(host), false);
    })
  });

  // Frees used memory
  hvm::collect(&heap, &prog.aris, tids[0], hvm::load_ptr(&heap, host));
  hvm::free(&heap, 0, 0, 1);
}

benchmark_example!(radix_sort, "../examples/sort/radix/main.hvm", "(Main 16)");
benchmark_example!(piadic_clifford, "../examples/lambda/padic_clifford/main.hvm", "(Main)");
benchmark_example!(lambda_multiplication, "../examples/lambda/multiplication/main.hvm", "(Main 16)");

criterion_group!(benches, radix_sort, piadic_clifford, lambda_multiplication);
criterion_main!(benches);
