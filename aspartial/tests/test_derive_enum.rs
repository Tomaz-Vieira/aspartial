use ::aspartial::AsPartial;

#[derive(AsPartial)]
#[aspartial(name = Blobs)]
#[aspartial(attrs(
    #[derive(::serde::Serialize)]
))]
enum SomeEnum {
    StringVariant(String),
}

#[test]
fn test_derive_enum(){

    // let raw = serde_json::json!( {} );
    // let parsed:  PartialSomeStruct = serde_json::from_value(raw).unwrap();
    // assert_eq!(parsed.normal_string_field, None);
    // assert_eq!(parsed.defaults_to_7, 7);
    // assert_eq!(parsed.defaults_to_default, <bool as Default>::default());
}
