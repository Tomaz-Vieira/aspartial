#![allow(non_snake_case)]

use proc_macro::TokenStream;

mod syn_extensions;
mod as_partial;
mod serde_attributes;
mod derive_config;
mod util;

/// Generates a 'partial' version of the annotated type and implements `::aspartial::AsPartial`
/// for the annotated type.
///
/// # Attributes
/// ## `aspartial(name = MyPartial)`
/// Determines the name of the generated partial type. Required if 'newtype' is not specified.
///
/// ## `aspartial(newtype)`
/// Derive `::aspartial::AsPartial` setting the associated type `Partial` to be the same type
/// as the only field in this struct. Required if 'name' is not specified.
///
/// ## `aspartial(attrs(#[some_attr1] #[some_attr2]))`
/// Optional. Appends the specified attributes to the generated partial struct
#[proc_macro_derive(AsPartial, attributes(aspartial))]
pub fn derive_as_partial(input: TokenStream) -> TokenStream {
    match as_partial::do_derive_as_partial(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}
