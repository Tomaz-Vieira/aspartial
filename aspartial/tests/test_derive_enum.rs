use ::aspartial::AsPartial;

#[derive(AsPartial)]
#[aspartial(name = PartialSomeStruct)]
#[aspartial(attrs( #[derive(PartialEq, Eq, Debug, ::serde::Deserialize)] ))]
#[allow(dead_code)]
struct SomeStruct {
    a: u32,
    b: String,
}

#[derive(AsPartial)]
#[aspartial(name = PartialSomeEnum)]
#[aspartial(attrs( #[derive(::serde::Deserialize)] ))]
#[aspartial(attrs( #[serde(try_from="::serde_json::Value")] ))]
#[aspartial(attrs( #[serde(tag = "variant_tag")] ))]
#[aspartial(derive(TryFrom<::serde_json::Value>))]
#[allow(dead_code)]
enum SomeEnum {
    StringVariant(String),
    StructVariant(SomeStruct)
}

#[test]
fn test_derive_enum(){
    let raw = serde_json::json!(
        {
            "variant_tag": "StructVariant",
            "a": 1234,
            "b": "some string",
        }
    );
    let parsed: PartialSomeEnum = serde_json::from_value(raw).unwrap();
    let expected = PartialSomeStruct{a: Some(1234), b: Some("some string".to_owned())};
    assert_eq!(parsed.struct_variant, Some(expected));
}
