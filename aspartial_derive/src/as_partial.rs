use proc_macro2::Span;
use quote::quote;
use syn::{parse_quote, parse_quote_spanned, spanned::Spanned};
use proc_macro::TokenStream;

use crate::derive_config::ConfigsForAsPartial;
use crate::syn_extensions::{IAttrExt, IEnumExt, IVariantExt};
use crate::serde_attributes::SerdeEnumTagParams;

fn where_clause_for_partial<'field>(
    original_where: Option<syn::WhereClause>,
    fields: impl IntoIterator<Item=&'field syn::Field>,
) ->syn::WhereClause {
    let mut wc = original_where.unwrap_or(parse_quote!(where));

    let comma = parse_quote!(,);
    if !wc.predicates.empty_or_trailing() {
        wc.predicates.push_punct(comma);
    }

    for field in fields.into_iter() {
        let span = field.ty.span();
        let field_ty = &field.ty;
        wc.predicates.push_value( match cfg!(feature="serde") {
            true => parse_quote_spanned!{ span=> #field_ty : ::aspartial::AsPartial<Partial: ::serde::de::DeserializeOwned> },
            false => parse_quote_spanned!{ span=> #field_ty: ::aspartial::AsPartial },
        });
        wc.predicates.push_punct(comma);
        if field.attrs.iter().any(|attr| attr.is_serde_regular_default()) {
            let default_pred: syn::WherePredicate = parse_quote!(#field_ty: std::default::Default);
            wc.predicates.push_value(default_pred);
            wc.predicates.push_punct(comma);
        }
    }
    wc
}

pub fn make_partial_enum(input: &syn::ItemEnum) -> syn::Result<TokenStream>{
    let confs = ConfigsForAsPartial::from_attrs(&input.attrs)?;

    // if let Some(from_json_val) = &confs.derive_from_json_value {
    //     if !confs.attrs.iter().any(|attr| attr.is__serde__try_from__json_value()) {
    //         return Err(syn::Error::new(
    //             from_json_val.derive_key.span(),
    //             "auto deriving TryFrom<::serde_json::Value> needs #[serde(try_from='::serde_json::Value')]"
    //         ))
    //     }
    // }

    let enum_tag_style = SerdeEnumTagParams::from_attributes(&input.attrs);

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
                .. #empty_partial
            },)*
            _ => #empty_partial
        }
    };
    let partial_from_outer_tagged = quote! {
        { 'from_outer_tagged: {
            let empty = #empty_partial;
            #(if let Some(payload) = value.get(#variant_tags) {
                break 'from_outer_tagged Self{
                    #partial_struct_field_idents: ::serde_json::from_value(payload.clone()).ok(),
                    .. empty
                }
            })*
            break 'from_outer_tagged #partial_from_value
        }}
    };

    let partial_type_ident = &confs.partial_type_ident;
    let partial_type_attrs = confs.attrs;
    let partial_struct_fields: Vec<syn::Field> = input.variants.iter()
        .map(|v| v.as_partial_field())
        .collect::<syn::Result<_>>()?;
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
    let fields = input.variants.iter()
        .map(|v| v.fields())
        .flatten();
    let where_clause = where_clause_for_partial(input.generics.where_clause.clone(), fields);
    let enum_ident = &input.ident;

    let impl__TryFrom__json_value = cfg!(feature="serde").then_some(match enum_tag_style{
        SerdeEnumTagParams::Untagged => quote!{
            impl<#impl_generics> TryFrom<::serde_json::Value> for #partial_type_ident {
                type Error = ::serde_json::Error;
                fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                    Ok(#partial_from_value)
                }
            }
        },
        SerdeEnumTagParams::InternallyTagged { tag_key } => quote!{
            impl<#impl_generics> TryFrom<::serde_json::Value> for #partial_type_ident {
                type Error = ::serde_json::Error;
                fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                    let tag = match value.get(#tag_key) {
                        Some(::serde_json::Value::String(tag)) => tag,
                        _ => return Ok(#partial_from_value),
                    };
                    Ok(#partial_from_tag)
                }
            }
        },
        SerdeEnumTagParams::AdjacentlyTagged { tag_key, content_key } => quote!{
            impl<#impl_generics> TryFrom<::serde_json::Value> for #partial_type_ident {
                type Error = ::serde_json::Error;
                fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                    let value = value.get(#content_key).unwrap_or(value.clone());
                    let tag = match value.get(#tag_key) {
                        Some(::serde_json::Value::String(tag)) => tag,
                        _ => return Ok(#partial_from_value),
                    };
                    Ok(#partial_from_tag)
                }
            }
        },
        SerdeEnumTagParams::ExternallyTagged => quote! {
            impl<#impl_generics> TryFrom<::serde_json::Value> for #partial_type_ident {
                type Error = ::serde_json::Error;
                fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                    Ok(#partial_from_outer_tagged)
                }
            }
        }
    });
    let partial_derive_deserialize = cfg!(feature="serde").then_some(quote!(
        #[derive(::serde::Deserialize)]
        #[serde(bound = "")]
        #[serde(try_from="::serde_json::Value")]
    ));

    let expanded = quote!{
        impl #impl_generics ::aspartial::AsPartial for #enum_ident #ty_generics
            #where_clause
        {
            type Partial = #partial_type_ident #impl_generics;
        }

        impl #impl_generics ::aspartial::AsPartial for #partial_type_ident #ty_generics
            #where_clause
        {
            type Partial = #partial_type_ident #impl_generics;
        }

        #partial_derive_deserialize
        #(#partial_type_attrs)*
        pub struct #partial_type_ident #impl_generics
            #where_clause
        {
            #(#partial_struct_fields),*
        }

        #impl__TryFrom__json_value
    };
    Ok(proc_macro::TokenStream::from(expanded))
}

pub fn make_partial_struct(input: &syn::ItemStruct) -> syn::Result<TokenStream>{
    let confs = ConfigsForAsPartial::from_attrs(&input.attrs)?;

    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
    let where_clause = where_clause_for_partial(
        input.generics.where_clause.clone(),
        input.fields.iter()
    );

    let partial_struct = {
        let mut partial_struct = input.clone();
        partial_struct.ident = confs.partial_type_ident;
        partial_struct.attrs = confs.attrs;
        if cfg!(feature="serde") {
            partial_struct.attrs.push( parse_quote!( #[derive(::serde::Deserialize)] ));
            partial_struct.attrs.push( parse_quote!( #[serde(bound = "")] ));
        }
        partial_struct.generics.where_clause = Some(where_clause.clone());

        for field in partial_struct.fields.iter_mut() {
            field.vis = parse_quote!(pub);
            if cfg!(feature="serde") {
                field.attrs.retain(|attr| attr.is_serde_attr());
                let is_default_field = field.attrs.iter().any(|attr| attr.is_serde_any_default());
                if !is_default_field{ //fields with #[serde(default)] don't need to have type Option<_>
                    let field_ty = &field.ty;
                    field.attrs.push(parse_quote!(#[serde(default)]));
                    field.ty = parse_quote!(Option< <#field_ty as ::aspartial::AsPartial>::Partial >);
                }
            } else {
                field.attrs = vec![];
                let field_ty = &field.ty;
                field.ty = parse_quote!(Option< <#field_ty as ::aspartial::AsPartial>::Partial >);
            }
        }
        partial_struct
    };
    let struct_name = &input.ident;
    let partial_struct_name = &partial_struct.ident;
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

