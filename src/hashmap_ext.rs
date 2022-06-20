use std::hash::Hash;

use bevy::utils::HashMap;

pub trait HashMultiMapExt<K, T> {
    fn add_element(&mut self, key: K, value: T);
    fn extend_elements<I: IntoIterator<Item=(K, T)>>(&mut self, it: I) {
        for (key, value) in it {
            self.add_element(key, value);
        }
    }
}

impl<K: Eq + Hash, T> HashMultiMapExt<K, T> for HashMap<K, Vec<T>> {
    fn add_element(&mut self, key: K, value: T) {
        if let Some(list) = self.get_mut(&key) {
            list.push(value);
        } else {
            self.insert(key, vec![value]);
        }
    }
}