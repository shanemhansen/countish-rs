use std::collections::HashMap;

#[derive(Debug)]
pub struct Entry {
    key:  String,
    frequency: f64,
}

#[derive(Default,Debug)]
pub struct NaiveSampler {
    n: u64,
    vals: HashMap<String, u64>
}

impl NaiveSampler {
    pub fn observe(&mut self,key: &str) {
        self.n+=1;
        *self.vals.entry(key.to_string()).or_insert(0)+=1;
    }
    pub fn items_above_threshold(&self,val: f64) -> Vec<Entry> {
        let count : u64 = ((self.n as f64)*val) as u64;
        let mut entries = vec![];
        
        for (key, valref) in self.vals.iter() {
            let val = *valref;
            if val >= count {
                entries.push(Entry{key:key.clone(), frequency:val as f64/self.n as f64})
            }
        }
        entries
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let mut sampler = ::NaiveSampler{..Default::default()};
        for _ in 1..10 {
            sampler.observe("shane");
        }
        sampler.observe("hansen");
        let items = sampler.items_above_threshold(0.5);
        assert_eq!(items.len(),1);
        assert_eq!(items[0].key, "shane");            
    }
}
