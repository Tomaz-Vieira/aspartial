use ::aspartial::AsPartial;
#[derive(AsPartial, PartialEq, Eq, Debug, serde::Serialize)]
struct SomeStruct {
    #[serde(default = "default_number")]
    a: u32,
    b: String,
}

fn default_number() -> u32 {
    7
}

#[test]
fn test_derive_simple(){

    let raw = serde_json::json!(
        { "a": 123, "b": "lala" }
    );
    let parsed:  PartialSomeStruct = serde_json::from_value(raw).unwrap();
    assert_eq!(parsed.a, 123);
    assert_eq!(parsed.b, Some("lala".into()));

    let raw = serde_json::json!(
        { "a": 123 }
    );
    let parsed:  PartialSomeStruct = serde_json::from_value(raw).unwrap();
    assert_eq!(parsed.a, 123);
    assert_eq!(parsed.b, None);

    let raw = serde_json::json!(
        { "b": "lala"}
    );
    let parsed:  PartialSomeStruct = serde_json::from_value(raw).unwrap();
    assert_eq!(parsed.a, 7);
    assert_eq!(parsed.b, Some("lala".into()));
}
