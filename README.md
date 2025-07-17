# aspartial

Types that represent some type in a serialized payload can implement
`AsPartial` to specify what that structure would look like when incomplete.

```rust
use ::aspartial::AsPartial;

// A struct like this...
struct MyStruct{
  field1: Something,
  field2: String,
}
// ...would have a 'partial' representation like this, usually generated
// via #[derive(AsPartial)].
struct PartialMyStruct{
  field1: Option<<Something as AsPartial>::Partial>,
  field2: Option<<String as AsPartial>::Partial>,
}
// And an enum like this...
enum MyEnum{
  Something(Something),
  SomethingElse(String),
}
// ...would have a 'partial' representation like this, also usually
// auto-generated via #[derive(AsPartial)]
struct PartialMyEnum{
  something: Option< <Something as AsPartial>::Partial >,
  something_else: Option< <String as AsPartial>::Partial >,
}
// that is, the partial version of an enum doesn't really know which variant
// it represents (in fact, all variants could have identical fields), so a partial
// enum is a struct composed of all variants that may or may not exist.

// Note that each field type in in the original MyStruct and every variant
// in the original MyEnum must also implement AsPartial:
#[derive(AsPartial)]
#[aspartial(name = PartialSomething)]
struct Something{
  a: u32
}
```

Note that `AsPartial::Partial` also implements `AsPartial`, so that any
arbitrarily nested field is also allowed to be absent. This crate should provide
implementations for all primitive types.

