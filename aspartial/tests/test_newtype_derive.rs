#[derive(::aspartial::AsPartial)]
#[aspartial(name=PartialOuterStruct)]
#[aspartial(attrs(#[derive(PartialEq, Eq, Debug)]))]
#[allow(dead_code)]
struct OuterStruct{
    mid: MidStruct,
}

#[derive(::aspartial::AsPartial)]
#[aspartial(newtype)]
#[aspartial(attrs(#[derive(PartialEq, Eq, Debug)]))]
#[derive(::serde::Deserialize)]
struct MidStruct(InnerStruct);

#[derive(::aspartial::AsPartial, ::serde::Deserialize)]
#[aspartial(name = PartialInnerStruct)]
#[aspartial(attrs(#[derive(PartialEq, Eq, Debug)]))]
struct InnerStruct{
    a: u32,
    b: String,
}

#[test]
fn test_newtype_derive(){
    let raw_outer = serde_json::json!(
        {
            "mid": {
                "a": 123,
            }
        }
    );

    let parsed: PartialOuterStruct = serde_json::from_value(raw_outer).unwrap();
    let expected = PartialOuterStruct{
        mid: Some(PartialInnerStruct {
            a: Some(123),
            b: None
        })
    };
    assert_eq!(parsed, expected);
}
