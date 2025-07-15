use ::aspartial::AsPartial;

#[allow(dead_code)]
#[derive(AsPartial)]
#[aspartial(name = PartialSomeStruct)]
#[aspartial(attrs( #[derive(PartialEq, Eq, Debug)] ))]
#[derive(serde::Deserialize)]
struct SomeStruct {
    a: u32,
    b: String,
}

#[allow(dead_code)]
#[derive(AsPartial)]
#[aspartial(name = PartialSomeEnum)]
#[derive(::serde::Deserialize)]
#[serde(tag = "variant_tag")]
enum SomeEnum {
    #[serde(rename="bla")]
    Variant1(SomeStruct),
    Variant2(SomeStruct),
}

#[test]
fn test_derive_with_tag(){
    let raw = serde_json::json!(
        {
            "variant_tag": "bla",
            "a": 1234,
            "b": "some string",
        }
    );
    let parsed: PartialSomeEnum = serde_json::from_value(raw).unwrap();
    let expected_struct_variant = PartialSomeStruct{a: Some(1234), b: Some("some string".to_owned())};
    assert_eq!(parsed.variant1, Some(expected_struct_variant));
    assert_eq!(parsed.variant2, None);
}

#[allow(dead_code)]
#[derive(AsPartial)]
#[aspartial(name = PartialSomeEnum2)]
#[derive(::serde::Deserialize)]
#[serde(tag = "variant_tag", content = "the_content")]
enum SomeEnum2 {
    Variant1(SomeStruct),
    Variant2(SomeStruct),
}

#[test]
fn test_derive_enum_with_tag_and_content(){
    let raw = serde_json::json!(
        {
            "variant_tag": "Variant1",
            "the_content": {
                "a": 1234,
                "b": "some string",
            }
        }
    );
    let parsed: PartialSomeEnum2 = serde_json::from_value(raw).unwrap();
    let expected_struct_variant = PartialSomeStruct{a: Some(1234), b: Some("some string".to_owned())};
    assert_eq!(parsed.variant1, Some(expected_struct_variant));
    assert_eq!(parsed.variant2, None);
}
