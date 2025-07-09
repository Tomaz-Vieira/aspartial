use proc_macro2::Span;
use quote::{quote, format_ident};
use syn::{parse_quote, parse_quote_spanned, spanned::Spanned};
use proc_macro::TokenStream;

use crate::syn_extensions::{IAttrExt, IEnumExt, IVariantExt, IStructExt};
use crate::serde_attributes::SerdeEnumTagParams;

fn where_clause_for_partial(
    original_where: Option<syn::WhereClause>,
    field_types: impl IntoIterator<Item=&syn::Field>,
) ->syn::WhereClause {
    let mut wc = original_where.unwrap_or(parse_quote!(where));

    let comma = parse_quote!(,);
    if !wc.predicates.empty_or_trailing() {
        wc.predicates.push_punct(comma);
    }

    for field_ty in field_types.into_iter() {
        let span = field_ty.span();
        wc.predicates.push_value(parse_quote_spanned!{ span=>
            #field_ty: ::aspartial::util::AsSerializablePartial<Partial: std::clone::Clone + std::fmt::Debug>
        });
        wc.predicates.push_punct(comma);
    }
    wc
}

pub fn make_partial_enum(input: &syn::ItemEnum) -> syn::Result<TokenStream>{
    let tag_params = input.attrs.iter().find_map(SerdeEnumTagParams::try_from_attr);

    let global_rename_params = SerdeEnumTagParams::from_attributes(&input.attrs);

    let (partial_struct_field_idents, variant_tags): (Vec<syn::Ident>, Vec<syn::LitStr>) = input.tagged_variants()
        .map(|(tag, v)| (v.partial_field_name(), tag))
        .unzip();
    let empty_partial = quote!(Self{
        #(#partial_struct_field_idents: None),*
    });
    let partial_from_value = quote!(Self{
        #(#partial_struct_field_idents: ::serde_json::from_value(value.clone()).ok()),*
    });
    let partial_from_tag = quote!{
        match tag.as_str(){
            #(#variant_tags => Self{
                #partial_struct_field_idents: ::serde_json::from_value(value.clone()).ok(),
                .. #empty_partial,
            }),*
            _ => #empty_partial
        }
    };
    let partial_from_outer_tagged = quote! {
        { 'from_outer_tagged: {
            let empty = #empty_partial;
            #(if let Some(payload) = value.get(#variant_tags) {
                break 'from_outer_tagged Self{
                    #partial_struct_field_idents: ::serde_json::from_value(payload.clone()).ok(),
                    .. empty,
                }
            })*
        }}
    };

    let fn_tryFrom = match global_rename_params{
        SerdeEnumTagParams::Untagged => quote!{
            fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                Ok(#partial_from_value)
            }
        },
        SerdeEnumTagParams::InternallyTagged { tag_key } => quote!{
            fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                let tag = match value.get(#tag_key) {
                    Some(::serde_json::Value::String(tag)) => tag,
                    _ => return Ok(#partial_from_value),
                };
                Ok(#partial_from_tag)
            }
        },
        SerdeEnumTagParams::AdjacentlyTagged { tag_key, content_key } => quote!{
            fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                let value = value.get(#content_key).unwrap_or(value.clone());
                let tag = match value.get(#tag_key) {
                    Some(::serde_json::Value::String(tag)) => tag,
                    _ => return Ok(#partial_from_value),
                };
                Ok(#partial_from_tag)
            }
        },
        SerdeEnumTagParams::ExternallyTagged => quote! {
            fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                Ok(#partial_from_outer_tagged)
            }
        }
    };

    let partial_struct_ident = format_ident!("Partial{}", input.ident);
    let partial_struct_fields: Vec<syn::Field> = input.variants.iter()
        .map(|v| v.as_partial_field())
        .collect::<syn::Result<_>>()?;
    let (impl_generics, type_generics, _) = input.generics.split_for_impl();
    let field_types = input.variants.iter()
        .map(|v| v.field_types().into_iter())
        .flatten();
    let where_clause = where_clause_for_partial(input.generics.where_clause, field_types);

    let expanded = quote!{
        #[derive(Clone, Debug, ::serde::Deserialize)]
        #[serde(try_from = "::serde_json::Value")]
        pub struct #partial_struct_ident #impl_generics #where_clause
        {
            #(#partial_struct_fields),*
        }

        impl<#impl_generics> TryFrom<::serde_json::Value> for $partial_struct_ident {
            #fn_tryFrom
        }
    };
    Ok(proc_macro::TokenStream::from(expanded))
}

pub fn make_partial_struct(input: syn::ItemStruct) -> syn::Result<TokenStream>{
    let struct_name = &input.ident;
    let partial_struct = {
        let mut partial_struct = input.clone();
        // partial_struct.attrs.retain(|attr| attr.is_serde_attr());
        partial_struct.ident = format_ident!("Partial{struct_name}");
        partial_struct.attrs = vec![
            parse_quote!(#[derive(::serde::Serialize, ::serde::Deserialize)]),
            parse_quote!(#[serde(bound = "")]),
        ];
        partial_struct.generics.where_clause = {
            let mut wc = partial_struct.generics.where_clause.unwrap_or(parse_quote!(where));
            let comma = parse_quote!(,);

            if !wc.predicates.empty_or_trailing() {
                wc.predicates.push_punct(comma);
            }
            for field in partial_struct.fields.iter_mut() {
                let field_ty = &field.ty;
                let span = field_ty.span();
                wc.predicates.push_value(parse_quote_spanned!{ span=>
                    #field_ty: ::bioimg_spec::util::AsSerializablePartial<Partial: std::clone::Clone + std::fmt::Debug>
                });
                wc.predicates.push_punct(comma);

                field.attrs = std::mem::take(&mut field.attrs).into_iter()
                    .filter(|attr| {
                        attr.is_serde_attr()
                    })
                    .collect();
                if !field.attrs.iter().any(|a| a.is_serde_default()){
                    field.attrs.push(parse_quote!(#[serde(default)]));
                    field.ty = parse_quote!(Option< <#field_ty as ::bioimg_spec::util::AsPartial>::Partial >);
                }
            }
            Some(wc)
        };
        partial_struct
    };
    let partial_struct_name = &partial_struct.ident;

    let (impl_generics, ty_generics, where_clause) = partial_struct.generics.split_for_impl();
    let expanded = quote! {
        impl #impl_generics ::bioimg_spec::util::AsPartial for #struct_name #ty_generics
            #where_clause
        {
            type Partial = #partial_struct_name #impl_generics;
        }

        impl #impl_generics ::bioimg_spec::util::AsPartial for #partial_struct_name #ty_generics
            #where_clause
        {
            type Partial = Self;
        }

        #[derive(Clone, Debug)]
        #partial_struct
    };

    Ok(proc_macro::TokenStream::from(expanded))
}

pub fn do_derive_as_partial(input: TokenStream) -> syn::Result<TokenStream>{
    // Parse the input tokens into a syntax tree.
    // 

    let input = syn::parse::<syn::Item>(input)?;
    match input{
        syn::Item::Struct(input_struct) => {},
        syn::Item::Enum(input_enum) => {},
        _ => return Err(syn::Error::new(Span::call_site(), "Must apply to enum or struct"))
    }
}

// {
//     let input_struct = syn::parse::<syn::ItemStruct>(input)?;
//     let struct_name = &input_struct.ident;

//     let partial_struct = {
//         let mut partial_struct = input_struct.clone();
//         // partial_struct.attrs.retain(|attr| attr.is_serde_attr());
//         partial_struct.ident = format_ident!("Partial{struct_name}");
//         partial_struct.attrs = vec![
//             parse_quote!(#[derive(::serde::Serialize, ::serde::Deserialize)]),
//             parse_quote!(#[serde(bound = "")]),
//         ];
//         partial_struct.generics.where_clause = where_clause_for_partial(&input);
//         for field in partial_struct.fields.iter_mut() {
//             let field_ty = &field.ty;
//             field.attrs.retain(|attr| attr.is_serde_attr());
//             if !field.attrs.iter().any(|a| a.is_serde_default()){
//                 field.attrs.push(parse_quote!(#[serde(default)]));
//                 field.ty = parse_quote!(Option< <#field_ty as ::bioimg_spec::util::AsPartial>::Partial >);
//             }
//         }
//         partial_struct
//     };
//     let partial_struct_name = &partial_struct.ident;

//     let (impl_generics, ty_generics, where_clause) = partial_struct.generics.split_for_impl();
//     let expanded = quote! {
//         impl #impl_generics ::bioimg_spec::util::AsPartial for #struct_name #ty_generics
//             #where_clause
//         {
//             type Partial = #partial_struct_name #impl_generics;
//         }

//         impl #impl_generics ::bioimg_spec::util::AsPartial for #partial_struct_name #ty_generics
//             #where_clause
//         {
//             type Partial = Self;
//         }

//         #[derive(Clone, Debug)]
//         #partial_struct
//     };

//     // std::fs::write(format!("/tmp/blas__{}.rs", struct_name.to_string()), expanded.to_string()).unwrap();

//     Ok(proc_macro::TokenStream::from(expanded))
// }
