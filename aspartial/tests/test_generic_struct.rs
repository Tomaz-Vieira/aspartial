use std::borrow::Borrow;

use aspartial::AsPartial;

#[derive(AsPartial)]
#[aspartial(name = PartialSomething)]
#[aspartial(attrs( #[derive(PartialEq, Eq, Debug)] ))]
pub struct Something{
    b: String,
}

impl Borrow<str> for Something {
    fn borrow(&self) -> &str {
        self.b.borrow()
    }
}

#[derive(AsPartial)]
#[aspartial(name = PartialMyGeneric)]
pub struct MyGeneric<R>
where
    R: Borrow<str>
{
    pub generic_field: R,
    pub string_field: String,
}

#[test]
fn test_generic_structs(){
    let raw = serde_json::json!(
        {
            "generic_field": {
                "a": 123,
                "b": "lele"
            },
        }
    );
    let parsed:  PartialMyGeneric<Something> = serde_json::from_value(raw).unwrap();
    assert_eq!(parsed.generic_field, Some(PartialSomething{b: Some("lele".to_owned())}));
    assert_eq!(parsed.string_field, None);
}
