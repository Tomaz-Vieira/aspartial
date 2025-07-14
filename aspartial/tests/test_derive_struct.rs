use ::aspartial::AsPartial;

#[derive(AsPartial, serde::Serialize)]
#[aspartial(name=PartialSomeStruct)]
#[aspartial(attrs(
    #[derive(::serde::Deserialize)]
))]
#[allow(dead_code)]
struct SomeStruct {
    normal_string_field: String,
    #[serde(default = "_seven")]
    defaults_to_7: u32,
    #[serde(default)]
    defaults_to_default: bool,
}

fn _seven() -> u32 {
    7
}

#[test]
fn test_derive_struct(){
    let raw = serde_json::json!(
        {
        }
    );
    let parsed:  PartialSomeStruct = serde_json::from_value(raw).unwrap();
    assert_eq!(parsed.normal_string_field, None);
    assert_eq!(parsed.defaults_to_7, 7);
    assert_eq!(parsed.defaults_to_default, <bool as Default>::default());
}
