use std::{collections::HashMap, hash::Hash};

pub fn group_by<K, T, F, M>(items: Vec<T>, mapper: M, filter: F) -> HashMap<K, Vec<T>>
where
    K: Clone + Eq + Hash,
    M: Fn(&T) -> K,
    F: Fn((&K, &T)) -> bool,
{
    let mut grouped: HashMap<K, Vec<T>> = HashMap::new();
    for item in items {
        let key = mapper(&item);
        if filter((&key, &item)) {
            let key_grouped = match grouped.remove(&key) {
                Some(mut key_grouped) => {
                    key_grouped.push(item);
                    key_grouped
                }
                None => vec![item],
            };
            grouped.insert(key, key_grouped);
        }
    }

    grouped
}
