#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

/// A type that can have a "partial" or "incomplete" representation. These are
/// usually serializable types, and their "partial" representations are objects
/// with missing fields in some serialized format like JSON.
pub trait AsPartial{
    type Partial: AsPartial;
}

pub use ::aspartial_derive::AsPartial;

/// Partial types are mostly useful in the context of deserialization, to be able
/// to handle incomplete data in self-describing formats (e.g. JSON, YAML).
/// For convenience, the [AsSerializablePartial] trait is blanket-implemented
/// for all types that implement [AsPartial] and whose partial version is also
/// serializable
#[cfg(feature = "serde")]
pub trait AsSerializablePartial: AsPartial<Partial: serde::Serialize + serde::de::DeserializeOwned>
{}

#[cfg(feature = "serde")]
impl<T> AsSerializablePartial for T
where T: AsPartial<Partial: serde::Serialize + serde::de::DeserializeOwned>
{}

macro_rules! impl_AsPartial_as_Self { ( $type:ty ) => {
    impl AsPartial for $type{
        type Partial = Self;
    }
};}

impl_AsPartial_as_Self!(String);
impl_AsPartial_as_Self!(bool);
impl_AsPartial_as_Self!(usize);
impl_AsPartial_as_Self!(std::num::NonZeroUsize);
impl_AsPartial_as_Self!(u8);
impl_AsPartial_as_Self!(i8);
impl_AsPartial_as_Self!(u16);
impl_AsPartial_as_Self!(i16);
impl_AsPartial_as_Self!(u32);
impl_AsPartial_as_Self!(i32);
impl_AsPartial_as_Self!(u64);
impl_AsPartial_as_Self!(i64);
impl_AsPartial_as_Self!(u128);
impl_AsPartial_as_Self!(i128);
impl_AsPartial_as_Self!(f32);
impl_AsPartial_as_Self!((f32, f32));
impl_AsPartial_as_Self!(f64);
impl_AsPartial_as_Self!((f64, f64));

impl AsPartial for std::sync::Arc<str>{
    type Partial = String;
}

//FIXME: T::Partial and not Option<T::Partial>??
impl<T: AsPartial> AsPartial for Option<T>{
    type Partial = T::Partial;
}

impl<T: AsPartial> AsPartial for Vec<T> {
    type Partial = Vec<T::Partial>;
}

#[cfg(feature="serde")]
impl AsPartial for serde_json::Map<String, serde_json::Value>{
    type Partial = Self;
}

#[cfg(feature="iso8601")]
impl AsPartial for iso8601_timestamp::Timestamp {
    type Partial = String;
}
