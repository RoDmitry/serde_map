use ::core::fmt;
use ::core::marker::PhantomData;
use ::std::collections::HashMap;
use serde::de::{Deserialize, Deserializer, Error, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};

#[cfg(feature = "scylla")]
mod scylla;
#[cfg(feature = "typesense")]
mod typesense;

/// Helps to process data at the serialization/deserialization stage, before saving to the inner `Vec`.
/// Example:
/// ```rust
/// use serde::de::Error;
/// use serde_map::{SerdeMap, SerdeMapStrategy};
///
/// struct StringStrategy;
///
/// impl SerdeMapStrategy<String> for StringStrategy {
///     type Des = i64; // deserialized type
///     type SerRet<'s> = String; // serialization return type
///
///     fn serialize(d: &i64) -> Self::SerRet<'_> {
///         d.to_string()
///     }
///
///     fn deserialize<E: Error>(s: String) -> Result<Self::Des, E> {
///         s.parse().map_err(Error::custom)
///     }
/// }
///
/// type SerdeMapString<V> = SerdeMap<String, V, StringStrategy>; // note that `K` here is `String`
/// // but the inner `Vec` will contain only `i64`
/// ```
pub trait SerdeMapStrategy<Ser>: Sized {
    /// deserialized type
    type Des;
    /// serialization return type
    type SerRet<'s>: Serialize
    where
        Ser: 's;

    fn serialize(d: &Self::Des) -> Self::SerRet<'_>;

    fn deserialize<E: Error>(s: Ser) -> Result<Self::Des, E>;
}

/// Linear (one-to-one) serialization strategy
#[derive(Debug, Clone, Copy)]
pub struct Linear;

impl<Ser: Serialize> SerdeMapStrategy<Ser> for Linear {
    type Des = Ser;
    type SerRet<'s>
        = &'s Ser
    where
        Ser: 's;

    #[inline(always)]
    fn serialize(d: &Self::Des) -> Self::SerRet<'_> {
        d
    }

    #[inline(always)]
    fn deserialize<E>(s: Ser) -> Result<Self::Des, E> {
        Ok(s)
    }
}

#[derive(Debug, Clone)]
pub struct SerdeMap<K, V, KS: SerdeMapStrategy<K> = Linear>(pub Vec<(KS::Des, V)>, PhantomData<KS>);

impl<K, V, KS: SerdeMapStrategy<K>> SerdeMap<K, V, KS> {
    #[inline]
    pub fn new() -> Self {
        Self(Vec::new(), PhantomData)
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity), PhantomData)
    }

    #[inline]
    pub fn insert_unchecked(&mut self, k: KS::Des, v: V) {
        self.0.push((k, v));
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<K, V, KS: SerdeMapStrategy<K>> SerdeMap<K, Vec<V>, KS> {
    #[inline]
    pub fn push_to_same_last(&mut self, k: KS::Des, v: V)
    where
        KS::Des: PartialEq,
    {
        if let Some(last) = self.0.last_mut() {
            if last.0 == k {
                last.1.push(v);
                return;
            }
        }

        self.0.push((k, vec![v]));
    }
}

impl<K, V, KS: SerdeMapStrategy<K>> Default for SerdeMap<K, V, KS> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, KS: SerdeMapStrategy<K>> IntoIterator for SerdeMap<K, V, KS> {
    type Item = (KS::Des, V);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K, V, KS: SerdeMapStrategy<K>> IntoIterator for &'a SerdeMap<K, V, KS> {
    type Item = &'a (KS::Des, V);
    type IntoIter = std::slice::Iter<'a, (KS::Des, V)>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, K, V, KS: SerdeMapStrategy<K>> IntoIterator for &'a mut SerdeMap<K, V, KS> {
    type Item = &'a mut (KS::Des, V);
    type IntoIter = std::slice::IterMut<'a, (KS::Des, V)>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<K, V, KS: SerdeMapStrategy<K>> FromIterator<(KS::Des, V)> for SerdeMap<K, V, KS> {
    #[inline]
    fn from_iter<T: IntoIterator<Item = (KS::Des, V)>>(iter: T) -> Self {
        Self(iter.into_iter().collect(), PhantomData)
    }
}

impl<K, V, KS: SerdeMapStrategy<K>> From<Vec<(KS::Des, V)>> for SerdeMap<K, V, KS> {
    #[inline]
    fn from(data: Vec<(KS::Des, V)>) -> Self {
        Self(data, PhantomData)
    }
}

impl<K, V, KS: SerdeMapStrategy<K>, S> From<HashMap<KS::Des, V, S>> for SerdeMap<K, V, KS> {
    #[inline]
    fn from(hash: HashMap<KS::Des, V, S>) -> Self {
        let data = hash.into_iter().collect();
        Self(data, PhantomData)
    }
}

impl<K, V, KS: SerdeMapStrategy<K>, S> From<SerdeMap<K, V, KS>> for HashMap<KS::Des, V, S>
where
    <KS as SerdeMapStrategy<K>>::Des: std::cmp::Eq + std::hash::Hash,
    S: Default + std::hash::BuildHasher,
{
    #[inline]
    fn from(v: SerdeMap<K, V, KS>) -> Self {
        v.0.into_iter().collect()
    }
}

impl<K: Serialize, V: Serialize, KS: SerdeMapStrategy<K>> Serialize for SerdeMap<K, V, KS> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(&KS::serialize(k), v)?;
        }
        map.end()
    }
}

/// copied from `serde::de::impls`
macro_rules! map_impl {
    (
        $(#[$attr:meta])*
        $ty:ident <K $(: $kbound1:ident $(+ $kbound2:ident)*)*, V $(, $typaram:ident : $bound1:ident $(<$bound1_1:ident>)? $(+ $bound2:ident)*)*>, // added `$(<$bound1_1:ident>)?`
        $access:ident,
        $with_capacity:expr,
    ) => {
        $(#[$attr])*
        impl<'de, K, V $(, $typaram)*> Deserialize<'de> for $ty<K, V $(, $typaram)*>
        where
            K: Deserialize<'de> $(+ $kbound1 $(+ $kbound2)*)*,
            V: Deserialize<'de>,
            $($typaram: $bound1<$($bound1_1)?> $(+ $bound2)*),*
        {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct MapVisitor<K, V $(, $typaram: $bound1<$($bound1_1)?> $(+ $bound2)*)*> { // added `: $bound1 $(+ $bound2)*`
                    marker: PhantomData<$ty<K, V $(, $typaram)*>>,
                }

                impl<'de, K, V $(, $typaram)*> Visitor<'de> for MapVisitor<K, V $(, $typaram)*>
                where
                    K: Deserialize<'de> $(+ $kbound1 $(+ $kbound2)*)*,
                    V: Deserialize<'de>,
                    $($typaram: $bound1<$($bound1_1)?> $(+ $bound2)*),*
                {
                    type Value = $ty<K, V $(, $typaram)*>;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a map")
                    }

                    #[inline]
                    fn visit_map<A>(self, mut $access: A) -> Result<Self::Value, A::Error>
                    where
                        A: MapAccess<'de>,
                    {
                        let mut values = $with_capacity;

                        while let Some((key, value)) = $access.next_entry()? {
                            values.insert_unchecked(KS::deserialize(key)?, value);
                        }

                        Ok(values)
                    }
                }

                let visitor = MapVisitor { marker: PhantomData };
                deserializer.deserialize_map(visitor)
            }
        }
    }
}

map_impl! {
    SerdeMap<K, V, KS: SerdeMapStrategy<K> >,
    map,
    SerdeMap::new(),
}
