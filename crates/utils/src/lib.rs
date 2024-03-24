use std::sync::Mutex;

use essential_types::Hash;
use serde::Serialize;
use sha2::Digest;

pub struct Lock<T> {
    data: Mutex<T>,
}

impl<T> Lock<T> {
    pub fn new(data: T) -> Self {
        Lock {
            data: Mutex::new(data),
        }
    }

    pub fn apply<U>(&self, f: impl FnOnce(&mut T) -> U) -> U {
        f(&mut self.data.lock().unwrap())
    }
}

pub fn hash<T: Serialize>(t: &T) -> Hash {
    let data = postcard::to_allocvec(t).unwrap();
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    hasher.update(&data);
    hasher.finalize().into()
}
