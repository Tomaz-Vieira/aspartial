
use ::aspartial::AsPartial;

#[allow(dead_code)]
#[derive(AsPartial)]
#[aspartial(name = PartialSomeStruct)]
#[aspartial(attrs( #[derive(PartialEq, Eq, Debug)] ))]
#[derive(::serde::Deserialize)]
struct SomeStruct {
    a: u32,
    b: String,
}

#[allow(dead_code)]
#[derive(AsPartial)]
#[aspartial(name = PartialSomeEnum)]
#[derive(::serde::Deserialize)]
#[serde(tag = "untagged")]
enum SomeEnum {
    StringVariant(String),
    StructVariant(SomeStruct)
}

#[test]
fn test_derive_enum(){
    let raw = serde_json::json!(
        {
            "a": 1234,
            "b": "some string",
        }
    );
    let parsed: PartialSomeEnum = serde_json::from_value(raw).unwrap();
    let expected_struct_variant = PartialSomeStruct{a: Some(1234), b: Some("some string".to_owned())};
    assert_eq!(parsed.struct_variant, Some(expected_struct_variant));
    assert_eq!(parsed.string_variant, None);
}
