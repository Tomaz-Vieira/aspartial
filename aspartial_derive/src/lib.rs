#![allow(incomplete_features)]
// #![feature(proc_macro_diagnostic, adt_const_params)]
#![allow(non_snake_case)]

use proc_macro::TokenStream;

mod syn_extensions;
mod as_partial;
mod serde_attributes;
mod derive_config;
mod util;

////////////////////////////////////////////

/// Generates a 'partial' version of the annotated type and implenets AsPartial
/// for the annotated type.
///
/// # Attributes
/// ## `aspartial(name = MyPartial)`
/// Required. Determines the name of the generated partial type.
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
