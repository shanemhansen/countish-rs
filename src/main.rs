extern crate countish;
extern crate getopts;

use getopts::Options;
use std::env;
use countish::{new_lossy_counter, Counter, new_naive_sampler, new_sampler, Entry};
use std::io::prelude::*;
use std::io;
fn process<T: Counter>(mut counter: T, threshold: f64) -> Vec<Entry> {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        if let Ok(line) = line {
            counter.observe(&line);
        }
    }
    counter.items_above_threshold(threshold)
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

pub fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optflag("h", "help", "print the help menu");
    opts.optopt("",
                "support",
                "",
                "Base granularity. Eg. if your support is .1 you can't find content that occurs \
                 at frequency .01");
    opts.optopt("",
                "error-tolerance",
                "",
                "Tolerable error (eg .01 for 1%). Impls:stucky, lossy");
    opts.optopt("",
                "failure-prob",
                "",
                "Chances that incorrect results will be published. Impls:sticky");
    opts.optopt("",
                "threshold",
                "",
                "frequency threshold: return entries who's frequency exceeds this.");
    opts.optopt("", "impl", "", "One of sticky|naive|lossy");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let (mut support, mut error_tolerance, mut failure_prob, mut threshold) =
        (0.0001, 0.0001, 0.0001, 0.05);
    let mut implementation = "sticky".to_string();
    if let Some(val) = matches.opt_str("support") {
        support = val.parse().unwrap();
    }
    if let Some(val) = matches.opt_str("error-tolerance") {
        error_tolerance = val.parse().unwrap();
    }
    if let Some(val) = matches.opt_str("failure-prob") {
        failure_prob = val.parse().unwrap();
    }
    if let Some(val) = matches.opt_str("threshold") {
        threshold = val.parse().unwrap();
    }
    if let Some(m) = matches.opt_str("impl") {
        implementation = m;
    }
    let entries = match implementation.as_ref() {
        "lossy" => process(new_lossy_counter(support, error_tolerance), threshold),
        "sticky" => {
            process(new_sampler(support, error_tolerance, failure_prob),
                    threshold)
        }
        "naive" => process(new_naive_sampler(), threshold),
        _ => panic!("unknown implementation"),

    };

    for entry in entries {
        println!("{} {}", entry.key, entry.frequency);
    }
}
