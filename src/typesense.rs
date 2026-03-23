use crate::{SerdeMap, SerdeMapStrategy};

impl<K, V, KS: SerdeMapStrategy<K>> typesense::prelude::ToTypesenseField for SerdeMap<K, V, KS> {
    #[inline(always)]
    fn to_typesense_type() -> &'static str {
        "object"
    }
}
