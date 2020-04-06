extern crate regex;

use regex::Regex;

use criterion::{criterion_group, Criterion};

fn is_numeric(num_str: &String) -> bool {
    if num_str.is_empty() {
        return false;
    }

    let first_char = num_str.chars().nth(0).unwrap();

    if !(first_char.is_numeric() || first_char == '-') {
        return false;
    }

    let is_num = num_str.parse::<f64>().is_ok();
    return is_num;
}

fn is_numeric_with_regex(num_str: &String) -> bool {
    if num_str.is_empty() {
        return false;
    }
    let re = Regex::new(r"^(([1-9]*)|(([1-9]*)\.([0-9]*)))$").unwrap();
    re.is_match(num_str)
}

fn is_numeric_with_regex_benchmark(c: &mut Criterion) {
    let float_str = String::from("-8.5");
    c.bench_function("is_numeric_with_regex", |b| b.iter(|| is_numeric_with_regex(&float_str)));
}

fn is_numeric_benchmark(c: &mut Criterion) {
    let float_str = String::from("-8.5");
    c.bench_function("is_numeric", |b| b.iter(|| is_numeric(&float_str)));
}

fn is_numeric_with_regex_benchmark_with_error(c: &mut Criterion) {
    let float_str = String::from("8e7983m2314081239840u");
    c.bench_function("is_numeric_with_regex", |b| b.iter(|| is_numeric_with_regex(&float_str)));
}

fn is_numeric_benchmark_with_error(c: &mut Criterion) {
    let float_str = String::from("8e7983m2314081239840u");
    c.bench_function("is_numeric", |b| b.iter(|| is_numeric(&float_str)));
}


criterion_group!(benches, is_numeric_with_regex_benchmark,is_numeric_benchmark,is_numeric_with_regex_benchmark_with_error,is_numeric_benchmark_with_error);