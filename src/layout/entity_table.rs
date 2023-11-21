use std::borrow::Borrow;
use std::collections::{hash_map, HashMap, HashSet};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

type EntityTableType<T> = HashMap<String, HashMap<T, HashSet<usize>>>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityTable<T: Serialize + Eq + Hash>(EntityTableType<T>);

impl<T> EntityTable<T>
where
    T: Serialize + Eq + Hash + ToOwned<Owned = T> + Default,
{
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert_entity<I: ?Sized>(&mut self, i: usize, entity: &str, value: &I)
    where
        T: Borrow<I>,
        I: Hash + Eq + ToOwned<Owned = T>,
    {
        if let Some(val_map) = self.0.get_mut(entity) {
            if let Some(set) = val_map.get_mut(value) {
                set.insert(i);
            } else {
                val_map.insert(value.to_owned(), HashSet::from([i]));
            }
        } else {
            let mut val_map = HashMap::new();
            val_map.insert(value.to_owned(), HashSet::from([i]));
            self.0.insert(entity.to_owned(), val_map);
        }
    }
    pub fn extend_entities<I, Q>(&mut self, entity: &str, value: &Q, ixs: I)
    where
        I: IntoIterator<Item = usize>,
        T: Borrow<Q>,
        Q: Hash + Eq + ToOwned<Owned = T> + ?Sized,
    {
        if let Some(val_map) = self.0.get_mut(entity) {
            if let Some(set) = val_map.get_mut(value) {
                set.extend(ixs);
            } else {
                val_map.insert(value.to_owned(), ixs.into_iter().collect());
            }
        } else {
            let mut val_map = HashMap::new();
            val_map.insert(value.to_owned(), ixs.into_iter().collect());
            self.0.insert(entity.to_owned(), val_map);
        }
    }
}

impl<T> From<EntityTableType<T>> for EntityTable<T>
where
    T: Serialize + Eq + Hash,
{
    fn from(value: EntityTableType<T>) -> Self {
        Self(value)
    }
}

impl<T> Deref for EntityTable<T>
where
    T: Serialize + Eq + Hash,
{
    type Target = EntityTableType<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for EntityTable<T>
where
    T: Serialize + Eq + Hash,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> IntoIterator for EntityTable<T>
where
    T: Serialize + Eq + Hash,
{
    type Item = (String, HashMap<T, HashSet<usize>>);
    type IntoIter = hash_map::IntoIter<String, HashMap<T, HashSet<usize>>>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
