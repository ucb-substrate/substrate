use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use super::Script;
use crate::component::serialize_params;

#[derive(Eq, PartialEq, Hash)]
struct Key {
    t: TypeId,
    params: Vec<u8>,
}

pub(crate) struct ScriptMap {
    map: HashMap<Key, Arc<dyn Any + Send + Sync>>,
}

impl ScriptMap {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn get<T>(&self, params: &T::Params) -> Option<Arc<T::Output>>
    where
        T: Script,
    {
        let params = serialize_params(params);
        self.get_with_buf::<T>(params)
    }

    pub(crate) fn get_with_buf<T>(&self, params: Vec<u8>) -> Option<Arc<T::Output>>
    where
        T: Script,
    {
        let key = Key {
            t: TypeId::of::<T>(),
            params,
        };

        let v = self.map.get(&key)?.clone();
        let v: Arc<T::Output> = Arc::downcast(v).unwrap();

        Some(v)
    }

    pub(crate) fn set<T>(&mut self, params: &T::Params, output: T::Output) -> Arc<T::Output>
    where
        T: Script,
    {
        let params = serialize_params(params);
        self.set_with_buf::<T>(params, output)
    }

    pub(crate) fn set_with_buf<T>(&mut self, params: Vec<u8>, output: T::Output) -> Arc<T::Output>
    where
        T: Script,
    {
        let key = Key {
            t: TypeId::of::<T>(),
            params,
        };

        let arc = Arc::new(output);
        self.map.insert(key, arc.clone());
        arc
    }
}
