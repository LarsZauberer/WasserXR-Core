use std::collections::HashMap;
use std::hash::Hash;

/// Syncs a map of tracked objects against a list of source items.
///
/// - `remove_stale` is called with the tracked object of every key that no longer
///   appears in `items`, after it has been removed from the map.
/// - `create_new` is called for every item without a tracked object; the result is
///   inserted into the map.
/// - `update` is called for every item afterwards, including freshly created ones.
///
/// `ctx` is shared mutable state (e.g. a scene or physics world) handed to each action.
pub(crate) fn sync_objects<Ctx, T, K, V>(
    ctx: &mut Ctx,
    map: &mut HashMap<K, V>,
    items: &[T],
    key: impl Fn(&T) -> K,
    mut create_new: impl FnMut(&mut Ctx, &T) -> V,
    mut remove_stale: impl FnMut(&mut Ctx, V),
    mut update: impl FnMut(&mut Ctx, &T, &mut V),
) where
    K: Eq + Hash + Copy,
{
    let stale_keys: Vec<K> = map
        .keys()
        .filter(|k| !items.iter().any(|item| key(item) == **k))
        .copied()
        .collect();

    for stale_key in stale_keys {
        if let Some(value) = map.remove(&stale_key) {
            remove_stale(ctx, value);
        }
    }

    for item in items {
        let value = map
            .entry(key(item))
            .or_insert_with(|| create_new(ctx, item));
        update(ctx, item, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_sync(map: &mut HashMap<u32, String>, items: &[u32]) -> Vec<String> {
        let mut log = Vec::new();
        sync_objects(
            &mut log,
            map,
            items,
            |item| *item,
            |log, item| {
                log.push(format!("create {item}"));
                format!("object-{item}")
            },
            |log, value| log.push(format!("remove {value}")),
            |log, item, value| log.push(format!("update {item} -> {value}")),
        );
        log
    }

    #[test]
    fn creates_new_objects() {
        let mut map = HashMap::new();
        let log = run_sync(&mut map, &[1, 2]);

        assert_eq!(map.get(&1), Some(&"object-1".to_owned()));
        assert_eq!(map.get(&2), Some(&"object-2".to_owned()));
        assert!(log.contains(&"create 1".to_owned()));
        assert!(log.contains(&"create 2".to_owned()));
    }

    #[test]
    fn removes_stale_objects() {
        let mut map = HashMap::new();
        map.insert(1, "object-1".to_owned());
        map.insert(2, "object-2".to_owned());

        let log = run_sync(&mut map, &[2]);

        assert!(!map.contains_key(&1));
        assert!(map.contains_key(&2));
        assert!(log.contains(&"remove object-1".to_owned()));
    }

    #[test]
    fn updates_existing_and_new_objects() {
        let mut map = HashMap::new();
        map.insert(1, "object-1".to_owned());

        let log = run_sync(&mut map, &[1, 2]);

        assert!(log.contains(&"update 1 -> object-1".to_owned()));
        assert!(log.contains(&"update 2 -> object-2".to_owned()));
        // Existing objects are kept, not recreated.
        assert!(!log.contains(&"create 1".to_owned()));
    }

    #[test]
    fn empty_items_clears_map() {
        let mut map = HashMap::new();
        map.insert(1, "object-1".to_owned());

        let log = run_sync(&mut map, &[]);

        assert!(map.is_empty());
        assert_eq!(log, vec!["remove object-1".to_owned()]);
    }
}
