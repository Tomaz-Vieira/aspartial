use proc_macro2::Span;
use quote::{quote, format_ident};
use syn::{parse_quote, parse_quote_spanned, spanned::Spanned};
use proc_macro::TokenStream;

use crate::syn_extensions::{IAttrExt, IEnumExt, IVariantExt};
use crate::serde_attributes::SerdeEnumTagParams;

fn where_clause_for_partial<'f>(
    original_where: Option<syn::WhereClause>,
    field_types: impl IntoIterator<Item=&'f syn::Type>,
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

    let partial_enum_ident = format_ident!("Partial{}", input.ident);
    let partial_struct_fields: Vec<syn::Field> = input.variants.iter()
        .map(|v| v.as_partial_field())
        .collect::<syn::Result<_>>()?;
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
    let field_types = input.variants.iter()
        .map(|v| v.field_types())
        .flatten();
    let where_clause = where_clause_for_partial(input.generics.where_clause.clone(), field_types);
    let enum_ident = &input.ident;

    let expanded = quote!{const _: () = {
        impl #impl_generics ::aspartial::AsPartial for #enum_ident #ty_generics
            #where_clause
        {
            type Partial = #partial_enum_ident #impl_generics;
        }

        #[derive(Clone, Debug, ::serde::Deserialize)]
        #[serde(try_from = "::serde_json::Value")]
        pub struct #partial_enum_ident #impl_generics
            #where_clause
        {
            #(#partial_struct_fields),*
        }

        impl<#impl_generics> TryFrom<::serde_json::Value> for $partial_struct_ident {
            #fn_tryFrom
        }
    }};
    Ok(proc_macro::TokenStream::from(expanded))
}

pub fn make_partial_struct(input: &syn::ItemStruct) -> syn::Result<TokenStream>{
    let struct_name = &input.ident;
    let partial_struct = {
        let mut partial_struct = input.clone();
        partial_struct.ident = format_ident!("Partial{struct_name}");
        partial_struct.attrs = vec![
            parse_quote!(#[derive(::serde::Serialize, ::serde::Deserialize)]),
            parse_quote!(#[serde(bound = "")]),
        ];
        for field in partial_struct.fields.iter_mut() {
            field.attrs.retain(|attr| attr.is_serde_attr());
            field.vis = parse_quote!(pub);
            if !field.attrs.iter().any(|a| a.is_serde_default()){
                let field_ty = &field.ty;
                field.attrs.push(parse_quote!(#[serde(default)]));
                field.ty = parse_quote!(Option< <#field_ty as ::aspartial::AsPartial>::Partial >);
            }
        }
        partial_struct
    };
    let partial_struct_name = &partial_struct.ident;
    let (impl_generics, ty_generics, where_clause) = partial_struct.generics.split_for_impl();
    let expanded = quote! {
        impl #impl_generics ::aspartial::AsPartial for #struct_name #ty_generics
            #where_clause
        {
            type Partial = #partial_struct_name #impl_generics;
        }

        impl #impl_generics ::aspartial::AsPartial for #partial_struct_name #ty_generics
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
    let input = syn::parse::<syn::Item>(input)?;
    let output = match input{
        syn::Item::Struct(input_struct) => make_partial_struct(&input_struct),
        syn::Item::Enum(input_enum) => make_partial_enum(&input_enum),
        _ => return Err(syn::Error::new(Span::call_site(), "Must apply to enum or struct"))
    }?;

    Ok(proc_macro::TokenStream::from(output))
}

