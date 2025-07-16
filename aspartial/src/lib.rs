/// Types that represent some type in a serialized payload can implement
/// AsPartial to specify what that structure would look like when incomplete.
/// 
/// ```rust
/// use ::aspartial::AsPartial;
/// 
/// // A struct like this...
/// struct MyStruct{
///   field1: Something,
///   field2: String,
/// }
///
/// // ...would have a 'partial' representation like this, usually generated
/// // via #[derive(AsPartial)].
/// struct PartialMyStruct{
///   field1: Option<<Something as AsPartial>::Partial>,
///   field2: Option<<String as AsPartial>::Partial>,
/// }
///
/// // And an enum like this...
/// enum MyEnum{
///   Something(Something),
///   SomethingElse(String),
/// }
///
/// // ...would have a 'partial' representation like this, also usually
/// // auto-generated via #[derive(AsPartial)]
/// struct PartialMyEnum{
///   something: Option< <Something as AsPartial>::Partial >,
///   something_else: Option< <String as AsPartial>::Partial >,
/// }
///
/// 
/// // Note that each field type in in the original MyStruct and every variant
/// // in the original MyEnum must also implement AsPartial:
/// #[derive(AsPartial)]
/// #[aspartial(name = PartialSomething)]
/// #[aspartial(attrs(
///     #[derive(::serde::Serialize)]
/// ))]
/// struct Something{
///   a: u32
/// }
/// ```
///
///
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
