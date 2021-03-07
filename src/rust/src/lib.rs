use extendr_api::prelude::*;
use lru::LruCache;
use regex::Regex;

#[extendr]
pub struct RR4R {
    cache: LruCache<String, Regex>,
}

impl RR4R {
    fn get_or_compile_regex(&mut self, pattern: &String) -> &Regex {
        if !self.cache.contains(pattern) {
            let re = Regex::new(pattern.as_str()).unwrap();
            self.cache.put(pattern.clone(), re);
        }

        self.cache.get(pattern).unwrap()
    }
}

#[extendr]
impl RR4R {
    fn new(cap: u16) -> Self {
        let cache = LruCache::new(cap.into());
        Self { cache }
    }

    fn resize(&mut self, cap: u16) {
        self.cache.resize(cap.into())
    }

    fn clear(&mut self) {
        self.cache.clear()
    }

    fn rr4r_detect(&mut self, x: Robj, pattern: String) -> Vec<Bool> {
        if x.is_na() {
            return vec![NA_LOGICAL];
        }
        let re = self.get_or_compile_regex(&pattern);

        let x_str_iter = x.as_str_iter().unwrap();
        x_str_iter
            .map(|s| {
                if s.is_na() {
                    return NA_LOGICAL;
                }
                re.is_match(&s).into()
            })
            .collect::<Vec<_>>()
    }

    fn rr4r_extract(&mut self, x: Robj, pattern: String) -> Vec<Option<&'static str>> {
        if x.is_na() {
            return vec![NA_STRING];
        }
        let re = self.get_or_compile_regex(&pattern);

        let x_str_iter = x.as_str_iter().unwrap();
        x_str_iter
            .map(|s| {
                if s.is_na() {
                    return NA_STRING;
                }

                if let Some(m) = re.find(&s) {
                    Some(m.as_str())
                } else {
                    NA_STRING
                }
            })
            .collect::<Vec<_>>()
    }

    fn rr4r_extract_all(&mut self, x: Robj, pattern: String) -> Robj {
        if x.is_na() {
            return List(NA_STRING).into();
        }
        let re = self.get_or_compile_regex(&pattern);

        let x_str_iter = x.as_str_iter().unwrap();
        let list_inner: Vec<Robj> = x_str_iter
            .map(|s| {
                if s.is_na() {
                    return NA_STRING.into_robj();
                }

                // This needs to be collected as Vec<String> first, otherwise map(AsRef::as_ref) cannot be applied,
                // though I don't understand why...
                let v: Vec<String> = re.captures_iter(&s).map(|cap| cap[0].to_string()).collect();

                v.iter().map(AsRef::as_ref).map(|s| Some(s)).collect_robj()
            })
            .collect();

        list_inner.into_robj()
    }

    fn rr4r_extract_groups(&mut self, x: Robj, pattern: String) -> Robj {
        // This function doesn't need to handle the case when `x` is NA specially

        let re = self.get_or_compile_regex(&pattern);
        let group_names: Vec<_> = re
            .capture_names()
            // First group is always for the whole match and doesn't have name
            .skip(1)
            // For groups that doesn't have names, automatically assign names like "X1"
            .enumerate()
            .map(|(i, s)| {
                if s.is_some() {
                    s.unwrap().to_string()
                } else {
                    format!("X{}", i + 1)
                }
            })
            .collect();
        let ncol = group_names.len();

        let mut tmp: Vec<Vec<Option<&str>>> = Vec::with_capacity(group_names.len());
        for _ in 0..ncol {
            tmp.push(Vec::with_capacity(x.len()));
        }

        if x.is_na() {
            for i in 0..ncol {
                tmp.get_mut(i).unwrap().push(NA_STRING);
            }
        } else {
            let x_str_iter = x.as_str_iter().unwrap();
            for s in x_str_iter {
                if s.is_na() {
                    for i in 0..ncol {
                        tmp.get_mut(i).unwrap().push(NA_STRING);
                    }
                    continue;
                }

                if let Some(cap) = re.captures(&s) {
                    for i in 0..ncol {
                        if let Some(m) = cap.get(i + 1) {
                            tmp.get_mut(i).unwrap().push(Some(m.as_str()));
                        } else {
                            tmp.get_mut(i).unwrap().push(NA_STRING);
                        }
                    }
                } else {
                    for i in 0..ncol {
                        tmp.get_mut(i).unwrap().push(NA_STRING);
                    }
                }
            }
        }

        // Create a list from Vec
        let result: Robj = tmp
            .into_iter()
            .map(|v| v.to_owned().into_robj())
            // To create a list, we need to craete Vec<Robj> first, then convert it to Robj.
            .collect::<Vec<Robj>>()
            .into_robj();

        result.set_names(group_names).unwrap()
    }
}

// Macro to generate exports
extendr_module! {
    mod rr4r;
    impl RR4R;
}