
use ::aspartial::AsPartial;

#[allow(dead_code)]
#[derive(AsPartial, serde::Deserialize)]
#[aspartial(name=PartialInner)]
#[aspartial(attrs( #[derive(PartialEq, Eq, Debug)] ))]
struct Inner{
    x: u32,
    y: String,
}


#[allow(dead_code)]
#[derive(AsPartial, serde::Deserialize)]
#[aspartial(name=PartialSomeStruct)]
struct Outer {
    a: String,
    #[serde(flatten)]
    b: Inner,
    c: u32,
}

#[test]
fn test_derive_flattened_struct(){
    let raw = serde_json::json!(
        {
            "a": "asd",
            "x": 123,
            "y": "some y"
        }
    );
    let parsed:  PartialSomeStruct = serde_json::from_value(raw).unwrap();
    assert_eq!(parsed.a, Some("asd".to_owned()));
    assert_eq!(parsed.b, Some(PartialInner{x: Some(123), y: Some("some y".to_owned())}));
    assert_eq!(parsed.c, None);
}
