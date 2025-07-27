use proc_macro2::Span;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote, parse_quote_spanned, spanned::Spanned};
use proc_macro::TokenStream;

use crate::derive_config::{ConfigsForAsPartial, ModeConfig};
use crate::syn_extensions::{IAttrExt, IEnumExt, IFieldExt, IVariantExt};
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
        wc.predicates.push_value(parse_quote_spanned!{span=>
            #field_ty : ::aspartial::AsPartial<Partial: ::serde::de::DeserializeOwned>
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

    let partial_type_ident = match confs.mode{
        ModeConfig::Name(conf) => conf.ident,
        ModeConfig::PartialIsInner(conf) => return Err(
            syn::Error::new(conf.partial_is_inner_keyword.span(), "Using inner as partial is only valid for newtype structs")
        )
    };
    let (partial_struct_field_idents, variant_tags): (Vec<syn::Ident>, Vec<syn::LitStr>) = input.tagged_variants()
        .map(|(tag, v)| (v.partial_field_name(), tag))
        .unzip();
    let empty_partial = quote!(#partial_type_ident{
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

    let impl__TryFrom__json_value = match enum_tag_style{
        SerdeEnumTagParams::Untagged => quote!{
            impl #impl_generics TryFrom<::serde_json::Value> for #partial_type_ident #ty_generics #where_clause {
                type Error = ::serde_json::Error;
                fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                    Ok(#partial_from_value)
                }
            }
        },
        SerdeEnumTagParams::InternallyTagged { tag_key } => quote!{
            impl #impl_generics TryFrom<::serde_json::Value> for #partial_type_ident #ty_generics #where_clause {
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
            impl #impl_generics TryFrom<::serde_json::Value> for #partial_type_ident #ty_generics #where_clause {
                type Error = ::serde_json::Error;
                fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                    let orig_val = &value;
                    let value = value.get(#content_key).unwrap_or(&value);
                    let tag = match orig_val.get(#tag_key) {
                        Some(::serde_json::Value::String(tag)) => tag,
                        _ => {
                            return Ok(#partial_from_value)
                        },
                    };
                    Ok(#partial_from_tag)
                }
            }
        },
        SerdeEnumTagParams::ExternallyTagged => quote! {
            impl #impl_generics TryFrom<::serde_json::Value> for #partial_type_ident #ty_generics #where_clause {
                type Error = ::serde_json::Error;
                fn try_from(value: ::serde_json::Value) -> Result<Self, Self::Error> {
                    Ok(#partial_from_outer_tagged)
                }
            }
        }
    };
    let partial_derive_deserialize = quote!(
        #[derive(::serde::Deserialize)]
        #[serde(bound = "")]
        #[serde(try_from="::serde_json::Value")]
    );

    for variant in input.variants.iter() {
        if variant.fields.len() > 1 {
            return Err(syn::Error::new(variant.fields.span(), "Only single, unnamed fields supported in variants for now"))
        }
    }
    let fn__to_partial: syn::ItemFn = {
        let match_arms: Vec<_> = input.variants.iter()
            .enumerate()
            .map(|(variant_idx, variant)| {
                let variant_ident = &variant.ident;
                let destructure_ident = format_ident!("variant_{variant_idx}");
                let partial_field_name = variant.partial_field_name();

                quote!{
                    Self::#variant_ident(#destructure_ident) => {
                        #partial_type_ident {
                            #partial_field_name: Some(#destructure_ident.to_partial()),
                            ..empty
                        }
                    }
                }
            })
            .collect();
        parse_quote!(
            fn to_partial(self) -> Self::Partial {
                let empty = #empty_partial;
                match self {
                    #(#match_arms),*
                }
            }
        )
    };

    let expanded = quote!{
        impl #impl_generics ::aspartial::AsPartial for #enum_ident #ty_generics
            #where_clause
        {
            type Partial = #partial_type_ident #impl_generics;
           #fn__to_partial
        }

        impl #impl_generics ::aspartial::AsPartial for #partial_type_ident #ty_generics
            #where_clause
        {
            type Partial = #partial_type_ident #impl_generics;
            fn to_partial(self) -> Self::Partial {
                self
            }
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

    let struct_name = &input.ident;
    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
    let where_clause = where_clause_for_partial(
        input.generics.where_clause.clone(),
        input.fields.iter()
    );

    let partial_struct_ident = match confs.mode {
        ModeConfig::Name(name_conf) => name_conf.ident,
        ModeConfig::PartialIsInner(_) => {
            let mut fields = input.fields.iter();
            let Some(field) = fields.next() else {
                return Err(syn::Error::new(input.ident.span(), "aspartial(newtype): Newtype structs must have exactly one field"))
            };
            if let Some(unexpected_field) = fields.next() {
                return Err(syn::Error::new(unexpected_field.span(), "aspartial(newtype): Newtype structs can only have a single field"))
            }
            if field.is_serde_default() {
                return Err(syn::Error::new(field.span(), "Deriving as newtype would lose serde default"))
            }
            let field_ty = &field.ty;

            return Ok(quote!(
                impl #impl_generics ::aspartial::AsPartial for #struct_name #ty_generics
                    #where_clause
                {
                    type Partial = <#field_ty as ::aspartial::AsPartial>::Partial;
                    fn to_partial(self) -> Self::Partial {
                        self.0.to_partial()
                    }
                }
            ).into())
        }
    };

    let mut default_functions = Vec::<syn::ItemFn>::new();
    let partial_struct = {
        let mut partial_struct = input.clone();
        partial_struct.ident = partial_struct_ident;
        partial_struct.attrs = confs.attrs;
        partial_struct.attrs.push( parse_quote!( #[derive(::serde::Deserialize)] ));
        partial_struct.attrs.push( parse_quote!( #[serde(bound = "")] ));
        partial_struct.generics.where_clause = Some(where_clause.clone());

        for (field_idx, field) in partial_struct.fields.iter_mut().enumerate() {
            field.vis = parse_quote!(pub);
            field.ty = field.partial_type();
            let mut fixed_attrs = Vec::<syn::Attribute>::new();
            for attr in &field.attrs {
                if !attr.is_serde_attr(){
                    continue;
                }
                let Some(default_path) = attr.as_serde_default_func_path() else {
                    fixed_attrs.push(attr.clone());
                    continue;
                };
                let default_func_name = {
                    let field_ident = field.ident.as_ref().map(|ident| ident.to_string()).unwrap_or(field_idx.to_string());
                    format_ident!("__default_for__{}__{}", partial_struct.ident, field_ident)
                };
                default_functions.push({
                    let field_ty = &field.ty;
                    parse_quote!{
                        #[allow(non_snake_case)]
                        fn #default_func_name() -> #field_ty {
                            ::aspartial::AsPartial::to_partial(#default_path())
                        }
                    }
                });
                fixed_attrs.push({
                    let serde_default_arg = syn::LitStr::new(&default_func_name.to_string(), field.span());
                    parse_quote!(
                        #[serde(default=#serde_default_arg)]
                    )
                });
            }
            field.attrs = fixed_attrs;
        }
        partial_struct
    };

    let partial_struct_name = &partial_struct.ident;

    let fn__to_partial: syn::ItemFn = {
        let field_inits = input.fields.iter()
            .enumerate()
            .map(|(field_idx, field)|{
                let field_ident: proc_macro2::TokenStream = match field.ident.clone(){
                    Some(ident) => ident.to_token_stream(),
                    None => {
                        syn::LitInt::new(&field_idx.to_string(), field.span()).to_token_stream()
                    },
                };
                if field.partial_is_optional() {
                    quote!{#field_ident : Some(self.#field_ident.to_partial())}
                } else {
                    quote!{#field_ident : self.#field_ident.to_partial()}
                }
            })
            .collect::<Vec<_>>();
        parse_quote!(
            fn to_partial(self) -> Self::Partial {
                #partial_struct_name {
                    #(#field_inits),*
                }
            }
        )
    };

    let expanded = quote! {
        impl #impl_generics ::aspartial::AsPartial for #struct_name #ty_generics
            #where_clause
        {
            type Partial = #partial_struct_name #impl_generics;
            #fn__to_partial
        }

        impl #impl_generics ::aspartial::AsPartial for #partial_struct_name #ty_generics
            #where_clause
        {
            type Partial = Self;
            fn to_partial(self) -> Self::Partial {
                self
            }
        }

        #partial_struct

        #(#default_functions)*
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

