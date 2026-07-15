use std::fmt::Debug;
use tracing::debug;

#[derive(Debug)]
pub struct Cache<A: PartialEq + Debug, T> {
    access_key: Option<A>,
    inner: Option<T>,
}

impl<A: PartialEq + Debug, T> Default for Cache<A, T> {
    fn default() -> Self {
        Self {
            access_key: Default::default(),
            inner: Default::default(),
        }
    }
}

impl<A: PartialEq + Debug, T> Cache<A, T> {
    /// Stores the value given and returns any previous values stored
    pub fn store(&mut self, access_key: A, value: T) -> Option<(A, T)> {
        let result = match (self.access_key.take(), self.inner.take()) {
            (None, Some(_)) | (Some(_), None) => {
                unreachable!("Both should be some or both should be None")
            }
            (None, None) => None,
            (Some(old_key), Some(old_value)) => Some((old_key, old_value)),
        };
        self.access_key = Some(access_key);
        self.inner = Some(value);
        result
    }

    pub fn get(&self, access_key: &A) -> Option<&T> {
        debug_assert_eq!(
            self.access_key.is_some(),
            self.inner.is_some(),
            "Should always have inner IFF there is a key"
        );
        if access_key == self.access_key.as_ref()? {
            self.inner.as_ref()
        } else {
            None
        }
    }

    /// Gets the existing value if possible otherwise uses `f` to provide the
    /// value, stores it and returns a reference to it
    pub fn get_or_insert(&mut self, access_key: &A, f: impl FnOnce() -> (A, T)) -> &T {
        if self.get(access_key).is_some() {
            self.get(access_key).expect("had to call again instead of using let Some because compiler wouldn't drop the reference to self")
        } else {
            // Not already available generate new value
            debug!("Inserting new value into cache with key: {access_key:?}");
            let (access_key, value) = f();
            self.store(access_key, value);
            self.inner.as_ref().expect("we just inserted a value")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_receive() {
        let mut cache: Cache<i32, &str> = Cache::default();

        let was_empty = cache.store(1, "First").is_none();
        assert!(was_empty);

        assert_eq!(cache.get(&1), Some(&"First"));

        assert_eq!(cache.get_or_insert(&2, || (2, "Second")), &"Second");
        assert_eq!(cache.get(&2), Some(&"Second"));

        assert!(cache.get(&1).is_none(), "Empty if not same key");

        assert_eq!(
            cache.get(&2),
            Some(&"Second"),
            "Value still present once key matches"
        );
    }
}
