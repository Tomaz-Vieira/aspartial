#![allow(incomplete_features)]
// #![feature(proc_macro_diagnostic, adt_const_params)]
#![allow(non_snake_case)]

use proc_macro::TokenStream;

mod syn_extensions;
mod as_partial;
mod serde_attributes;
mod util;

////////////////////////////////////////////

#[proc_macro_derive(AsPartial)]
pub fn derive_as_partial(input: TokenStream) -> TokenStream {
    match as_partial::do_derive_as_partial(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}
