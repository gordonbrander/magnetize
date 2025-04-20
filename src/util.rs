use std::collections::HashMap;

/// Group a list of key-value pairs into a HashMap of key-to-vector.
pub fn group<K, V>(pairs: Vec<(K, V)>) -> HashMap<K, Vec<V>>
where
    K: Eq + std::hash::Hash,
{
    let mut groups: HashMap<K, Vec<V>> = HashMap::new();

    pairs.into_iter().for_each(|(key, value)| {
        groups.entry(key).or_insert(Vec::new()).push(value);
    });

    groups
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
}
