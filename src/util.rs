use rand::rng;
use rand::seq::SliceRandom;
use std::collections::HashMap;

/// Group a list of key-value pairs into a HashMap of key-to-vector.
pub fn group<K, V>(pairs: Vec<(K, V)>) -> HashMap<K, Vec<V>>
where
    K: Eq + std::hash::Hash,
{
    let mut query: HashMap<K, Vec<V>> = HashMap::new();

    pairs.into_iter().for_each(|(key, value)| {
        query.entry(key).or_insert(Vec::new()).push(value);
    });

    query
}

/// Choose up to `max` random elements from items, mutating original vector.
pub fn random_choice<T>(mut items: Vec<T>, max: usize) -> Vec<T> {
    let mut rng = rng();
    items.shuffle(&mut rng);
    items.truncate(max);
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_empty() {
        let pairs: Vec<(i32, String)> = Vec::new();
        let result = group(pairs);
        assert!(result.is_empty());
    }

    #[test]
    fn test_group_single_pair() {
        let pairs = vec![(1, "a".to_string())];
        let result = group(pairs);

        assert_eq!(result.len(), 1);
        assert_eq!(result.get(&1), Some(&vec!["a".to_string()]));
    }

    #[test]
    fn test_group_multiple_pairs_with_same_key() {
        let pairs = vec![
            (1, "a".to_string()),
            (1, "b".to_string()),
            (1, "c".to_string()),
        ];

        let result = group(pairs);

        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get(&1),
            Some(&vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn test_group_multiple_keys() {
        let pairs = vec![
            (1, "a".to_string()),
            (2, "b".to_string()),
            (3, "c".to_string()),
            (1, "d".to_string()),
        ];

        let result = group(pairs);

        assert_eq!(result.len(), 3);
        assert_eq!(
            result.get(&1),
            Some(&vec!["a".to_string(), "d".to_string()])
        );
        assert_eq!(result.get(&2), Some(&vec!["b".to_string()]));
        assert_eq!(result.get(&3), Some(&vec!["c".to_string()]));
    }

    #[test]
    fn test_random_choice() {
        let items = vec![
            "http://example1.com",
            "http://example2.com",
            "http://example3.com",
            "http://example4.com",
            "http://example5.com",
        ];

        let selected = random_choice(items.to_owned(), 3);

        assert_eq!(selected.len(), 3);
        // Verify all selected peers are from the original list
        for item in &selected {
            assert!(items.contains(item));
        }
    }

    #[test]
    fn test_random_choice_with_max_greater_than_available() {
        let items = vec!["http://example1.com", "http://example2.com"];

        let selected = random_choice(items, 5);

        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_random_choice_empty() {
        let items: Vec<String> = vec![];

        let selected = random_choice(items, 3);

        assert!(selected.is_empty());
    }
}
