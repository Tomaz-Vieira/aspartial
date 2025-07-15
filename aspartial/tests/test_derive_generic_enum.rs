use aspartial::AsPartial;

#[derive(AsPartial)]
#[aspartial(name = PartialSomething)]
#[aspartial(attrs( #[derive(PartialEq, Eq, Debug)] ))]
pub struct Something{
    #[allow(dead_code)]
    b: u32,
}

#[derive(AsPartial)]
#[aspartial(name = PartialSingleOrMultiple)]
#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum SingleOrMultiple<T> {
    Single(T),
    Multiple(Vec<T>),
}

#[test]
fn test_derive_generic_enum(){
    let raw = serde_json::json!(
        {
            "b": 123
        }
    );
    let parsed: PartialSingleOrMultiple<Something> = serde_json::from_value(raw).unwrap();
    assert_eq!(parsed.multiple, None);
    assert_eq!(parsed.single, Some(PartialSomething { b: Some(123) }));

    let raw = serde_json::json!(
        [
            { "b": 123 },
            {}
        ]
    );
    let parsed: PartialSingleOrMultiple<Something> = serde_json::from_value(raw).unwrap();
    assert_eq!(
        parsed.multiple,
        Some(vec![
            PartialSomething { b: Some(123) },
            PartialSomething { b: None },
        ])
    )
}
