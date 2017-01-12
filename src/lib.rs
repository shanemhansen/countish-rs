#![deny(missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces, unused_qualifications)]
//! A collection of approximate frequency counting algorithms for rust
extern crate rand;
use std::collections::HashMap;
use rand::{thread_rng, Rng};


/// trait for things which can count values
pub trait Counter {
    /// `observe` tracks a value
    fn observe(&mut self, key: &str);
    /// `items_above_threshold` return entries above threshold
    fn items_above_threshold(&self, threshold: f64) -> Vec<Entry>;
}

/// `Entry` tracks a key and it's frequency.
#[derive(Debug)]
pub struct Entry {
    /// The observed value
    pub key: String,
    /// The approximate frequency of the observed value on the interval (0,1]
    pub frequency: f64,
}

/// `NaiveSampler` is a reference exact counting implementation. I requires O(n) memory.
#[derive(Default,Debug)]
pub struct NaiveSampler {
    n: u64,
    vals: HashMap<String, u64>,
}

/// Construct a new `NaiveSampler`
pub fn new_naive_sampler() -> NaiveSampler {
    NaiveSampler {
        n: 0,
        vals: HashMap::new(),
    }
}

impl Counter for NaiveSampler {
    /// record that the given key has been observed.
    fn observe(&mut self, key: &str) {
        self.n += 1;
        *self.vals.entry(key.to_string()).or_insert(0) += 1;
    }
    /// return items who's frequency exceeds threshld
    fn items_above_threshold(&self, threshold: f64) -> Vec<Entry> {
        let count: u64 = ((self.n as f64) * threshold) as u64;
        self.vals
            .iter()
            .filter(|&(_, valref)| *valref >= count)
            .map(|(key, valref)| {
                Entry {
                    key: key.clone(),
                    frequency: *valref as f64 / self.n as f64,
                }
            })
            .collect()
    }
}

#[derive(Debug)]
struct FDeltaPair {
    f: f64,
    delta: f64,
}


/// `LossyCounter` implements the lossy counter outlined here
/// http://www.vldb.org/conf/2002/S10P03.pdf
#[derive(Debug)]
pub struct LossyCounter {
    support: f64,
    d: HashMap<String, FDeltaPair>,
    n: u64,
    bucket_width: u64,
}

/// `new_lossy_counter` constructs a counter with the given support and error tolerance
pub fn new_lossy_counter(support: f64, error_tolerance: f64) -> LossyCounter {
    LossyCounter {
        support: support,
        d: HashMap::new(),
        bucket_width: (1.0 / error_tolerance).ceil() as u64,
        n: 0,
    }
}
impl LossyCounter {
    fn prune(&mut self, bucket: u64) {
        let fbucket = bucket as f64;
        let to_remove: Vec<String> = self.d
            .iter()
            .filter(|&(_, value)| value.f + value.delta <= fbucket)
            .map(|(key, _)| key.clone())
            .collect();
        for key in &to_remove {
            self.d.remove(key);
        }
    }
}
impl Counter for LossyCounter {
    /// return items who's frequency exceeds threshld
    fn items_above_threshold(&self, threshold: f64) -> Vec<Entry> {
        let f_n = self.n as f64;
        self.d
            .iter()
            .filter(|&(_, val)| val.f >= (threshold - val.delta) * f_n)
            .map(|(key, val)| {
                Entry {
                    key: key.clone(),
                    frequency: val.f / f_n + self.support,
                }
            })
            .collect()

    }
    /// record that the given key has been observed.
    fn observe(&mut self, key: &str) {
        self.n += 1;
        let bucket = (self.n / self.bucket_width) + 1;
        let newval = match self.d.get(key) {
            Some(val) => FDeltaPair { f: val.f + 1.0, ..*val },
            _ => {
                FDeltaPair {
                    f: 1.0,
                    delta: (bucket - 1) as f64,
                }
            }
        };
        self.d.insert(key.to_string(), newval);
        if self.n % self.bucket_width == 0 {
            self.prune(bucket);
        }
    }
}

/// `StickySampler` implements an approximate frequency counting algorithm outlined here
/// http://www.vldb.org/conf/2002/S10P03.pdf
#[derive(Debug)]
pub struct StickySampler {
    error_tolerance: f64,
    support: f64,
    s: HashMap<String, f64>,
    r: f64,
    n: f64,
    t: f64,
}

/// `new_sampler` returns a new sticky sampler with the given
/// `support`, `error_tolerance`, and failure probability
pub fn new_sampler(support: f64, error_tolerance: f64, failure_prob: f64) -> StickySampler {
    let two_t = 2.0 / error_tolerance * (1.0 / (support * failure_prob)).ln();
    StickySampler {
        error_tolerance: error_tolerance,
        support: support,
        r: 1.0,
        t: two_t,
        s: HashMap::new(),
        n: 0.0,
    }
}
impl StickySampler {
    fn prune(&mut self) {
        let mut rng = thread_rng();
        // TODO: clean this up. go allows mutations
        let mut to_remove: Vec<String> = vec![];
        let mut to_decr: Vec<String> = vec![];
        for (key, val) in &self.s {
            loop {
                if rng.gen_weighted_bool(2) {
                    break;
                }
                let mut myval = *val;
                myval -= 1.0;
                if myval <= 0.0 {
                    to_remove.push(key.clone());
                } else {
                    to_decr.push(key.clone());
                }
            }
        }
        for key in &to_remove {
            self.s.remove(key);
        }
        for key in &to_decr {
            *self.s.entry(key.clone()).or_insert(1.0) -= 1.0;
        }
    }
}
impl Counter for StickySampler {
    /// return items who's frequency exceeds threshld
    fn items_above_threshold(&self, threshold: f64) -> Vec<Entry> {
        self.s
            .iter()
            .filter(|&(_, f)| *f >= (threshold - self.error_tolerance) * self.n)
            .map(|(key, f)| {
                Entry {
                    key: key.clone(),
                    frequency: *f / self.n + self.support,
                }
            })
            .collect()
    }
    /// record that the given key has been observed.
    fn observe(&mut self, key: &str) {
        self.n += 1.0;
        let count = self.n;
        if count > self.t {
            self.t *= 2.0;
            self.r *= 2.0;
            self.prune()
        }
        if let Some(val) = self.s.get_mut(key) {
            *val += 1.0;
            return;
        } else {
            let mut rng = thread_rng();
            let should_sample = rng.next_f64() <= 1.0 / self.r;
            if !should_sample {
                return;
            }
        }
        // only arrive here for new elements which should be sampled
        let k = key.to_string();
        *self.s.entry(k).or_insert(0.0) += 1.0;
    }
}

#[cfg(test)]
mod tests {
    use ::Counter;
    #[test]
    fn naive() {
        let mut sampler = ::NaiveSampler { ..Default::default() };
        for _ in 1..10 {
            sampler.observe("shane");
        }
        sampler.observe("hansen");
        let items = sampler.items_above_threshold(0.5);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key, "shane");
    }
    #[test]
    fn lossy() {
        let mut sampler = ::new_lossy_counter(0.01, 0.005);
        for _ in 1..10 {
            sampler.observe("shane");
        }
        sampler.observe("hansen");
        let items = sampler.items_above_threshold(0.5);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key, "shane");
    }
    #[test]
    fn sticky() {
        let mut sampler = ::new_sampler(0.1, 0.1, 0.01);
        for _ in 1..10 {
            sampler.observe("shane");
        }
        sampler.observe("hansen");
        let items = sampler.items_above_threshold(0.5);
        assert_eq!(items.len(), 1, "asd");
        assert_eq!(items[0].key, "shane");
    }
}
