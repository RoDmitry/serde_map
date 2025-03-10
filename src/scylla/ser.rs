use crate::{SerdeMap, SerdeMapStrategy};
use scylla::{
    cluster::metadata::CollectionType,
    frame::response::result::ColumnType,
    serialize::{
        value::{
            BuiltinSerializationError, BuiltinSerializationErrorKind, BuiltinTypeCheckError,
            BuiltinTypeCheckErrorKind, MapSerializationErrorKind, MapTypeCheckErrorKind,
            SerializeValue,
        },
        writers::{CellWriter, WrittenCellProof},
        SerializationError,
    },
};

// copied from scylla
fn mk_ser_err_named(
    name: &'static str,
    got: &ColumnType,
    kind: impl Into<BuiltinSerializationErrorKind>,
) -> SerializationError {
    SerializationError::new(BuiltinSerializationError {
        rust_name: name,
        got: got.clone().into_owned(),
        kind: kind.into(),
    })
}

// copied from scylla
fn mk_typck_err_named(
    name: &'static str,
    got: &ColumnType,
    kind: impl Into<BuiltinTypeCheckErrorKind>,
) -> SerializationError {
    SerializationError::new(BuiltinTypeCheckError {
        rust_name: name,
        got: got.clone().into_owned(),
        kind: kind.into(),
    })
}

// copied from scylla
#[inline]
fn serialize_mapping<'t, 'b, K: SerializeValue + 't, V: SerializeValue + 't>(
    rust_name: &'static str,
    len: usize,
    iter: impl Iterator<Item = &'t (K, V)>,
    typ: &ColumnType,
    writer: CellWriter<'b>,
) -> Result<WrittenCellProof<'b>, SerializationError> {
    let (ktyp, vtyp) = match typ {
        ColumnType::Collection {
            frozen: false,
            typ: CollectionType::Map(k, v),
        } => (k, v),
        _ => {
            return Err(mk_typck_err_named(
                rust_name,
                typ,
                MapTypeCheckErrorKind::NotMap,
            ));
        }
    };

    let mut builder = writer.into_value_builder();

    let element_count: i32 = len.try_into().map_err(|_| {
        mk_ser_err_named(rust_name, typ, MapSerializationErrorKind::TooManyElements)
    })?;
    builder.append_bytes(&element_count.to_be_bytes());

    for (k, v) in iter {
        K::serialize(k, ktyp, builder.make_sub_writer()).map_err(|err| {
            mk_ser_err_named(
                rust_name,
                typ,
                MapSerializationErrorKind::KeySerializationFailed(err),
            )
        })?;
        V::serialize(v, vtyp, builder.make_sub_writer()).map_err(|err| {
            mk_ser_err_named(
                rust_name,
                typ,
                MapSerializationErrorKind::ValueSerializationFailed(err),
            )
        })?;
    }

    builder
        .finish()
        .map_err(|_| mk_ser_err_named(rust_name, typ, BuiltinSerializationErrorKind::SizeOverflow))
}

impl<K, V: SerializeValue, KS: SerdeMapStrategy<K>> SerializeValue for SerdeMap<K, V, KS>
where
    KS::Des: SerializeValue,
{
    fn serialize<'b>(
        &self,
        typ: &ColumnType,
        writer: CellWriter<'b>,
    ) -> Result<WrittenCellProof<'b>, SerializationError> {
        serialize_mapping(
            std::any::type_name::<Self>(),
            self.len(),
            self.0.iter(),
            typ,
            writer,
        )
    }
}
