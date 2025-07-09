/// Types that represent some structure in a serialized payload can implement
/// AsPartial to specify what that structure would look like when incomplete.
/// Usually, for a struct of the form
/// ```rust
/// struct MyStruct{
///   field1: Something,
///   field2: SomethingElse,
/// }
/// ```
/// a partial version of it would be of the form
/// ```rust
/// struct PartialMyStruct{
///   field1: Option<<Something as AsPartial>::Partial>,
///   field2: Option<<Something as AsPartial>::Partial>,
/// }
/// ```
///
///which expresses some data that has the same structure as MyStruct but
/// maybe have some (or all) of its (arbitrarily nested) fields missing.
///
/// For enums, an enum like
/// ```rust
/// enum MyEnum{
///   Something(Someting),
///   SomethingElse(SomethingElse),
/// }
/// ```
/// has a partial representaiton like this
/// ```rust
/// enum PartialMyEnum{
///   something: Option< <Something as AsPartial>::Partial >,
///   something_else: Option< <SomethingElse as AsPartial>::Partial >,
/// }
/// ```
///
/// that is, the partial version of an enum doesn't really know which variant
/// it represents, (in fact, all variants could have identical fields), so it is
/// composed of all possibilities that may or may not exist.
///
/// Note that [AsPartial::Partial] also implements [AsPartial], so that
/// any arbitrarily nested field can also be missing
pub trait AsPartial{
    type Partial: AsPartial;
}

pub use ::aspartial_derive::AsPartial;

/// Partial types are mostly useful in the context of (de)serialization, to be able
/// to handle incomplete data in self-describing formats (e.g. JSON, YAML).
/// For convenience, the AsSerializablePartial is blanket-implemented for all types
/// that implement `AsPartial` and whose partial version is also serializable
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
impl_AsPartial_as_Self!(usize);
impl_AsPartial_as_Self!(u32);
impl_AsPartial_as_Self!(std::num::NonZeroUsize);
impl_AsPartial_as_Self!(f32);
impl_AsPartial_as_Self!((f32, f32));
impl_AsPartial_as_Self!(bool);

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

#[cfg(feature="serde_json")]
impl AsPartial for serde_json::Map<String, serde_json::Value>{
    type Partial = Self;
}
