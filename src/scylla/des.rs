use crate::{SerdeMap, SerdeMapStrategy};
use scylla::deserialize::value::{DeserializeValue, MapIterator};
use scylla::deserialize::FrameSlice;
use scylla::errors::{DeserializationError, TypeCheckError};
use scylla::frame::response::result::ColumnType;

impl<'frame, 'metadata, K, V, KS> DeserializeValue<'frame, 'metadata> for SerdeMap<K, V, KS>
where
    V: DeserializeValue<'frame, 'metadata>,
    KS: SerdeMapStrategy<K>,
    KS::Des: DeserializeValue<'frame, 'metadata>,
{
    #[inline]
    fn type_check(typ: &ColumnType) -> Result<(), TypeCheckError> {
        MapIterator::<'frame, 'metadata, KS::Des, V>::type_check(typ)
        // .map_err(typck_error_replace_rust_name::<Self>)
    }

    #[inline]
    fn deserialize(
        typ: &'metadata ColumnType<'metadata>,
        v: Option<FrameSlice<'frame>>,
    ) -> Result<Self, DeserializationError> {
        MapIterator::<'frame, 'metadata, KS::Des, V>::deserialize(typ, v)
            .and_then(|it| it.collect::<Result<_, DeserializationError>>())
        // .map_err(deser_error_replace_rust_name::<Self>)
    }
}
