extern crate rand;
use std::collections::HashMap;
use rand::{thread_rng, Rng};

#[derive(Debug)]
pub struct Entry {
    pub key: String,
    pub frequency: f64,
}

#[derive(Default,Debug)]
pub struct NaiveSampler {
    n: u64,
    vals: HashMap<String, u64>,
}

pub fn new_naive_sampler() -> NaiveSampler {
    NaiveSampler {
        n: 0,
        vals: HashMap::new(),
    }
}

impl NaiveSampler {
    pub fn observe(&mut self, key: &str) {
        self.n += 1;
        *self.vals.entry(key.to_string()).or_insert(0) += 1;
    }
    pub fn items_above_threshold(&self, threshold: f64) -> Vec<Entry> {
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

struct FDeltaPair {
    f: f64,
    delta: f64,
}


pub struct LossyCounter {
    support: f64,
    d: HashMap<String, FDeltaPair>,
    n: u64,
    bucket_width: u64,
}
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
    pub fn items_above_threshold(&self, threshold: f64) -> Vec<Entry> {
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
    pub fn observe(&mut self, key: &str) {
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

pub struct StickySampler {
    error_tolerance: f64,
    support: f64,
    s: HashMap<String, f64>,
    r: f64,
    n: f64,
    t: f64,
}

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
    pub fn prune(&mut self) {
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
    pub fn items_above_threshold(&self, threshold: f64) -> Vec<Entry> {
        for (key, val) in &self.s {
            println!("{}:{}", key, val);
        }
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
    pub fn observe(&mut self, key: &str) {
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
