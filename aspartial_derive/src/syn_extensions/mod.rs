use quote::{quote, format_ident};
use syn::{parse_quote, parse_quote_spanned, spanned::Spanned};

use crate::serde_attributes::{SerdeDefaultAttrParams, SerdeEnumTagParams, SerdeInnerRenameParams, SerdeOuterRenameParams};

pub trait IAttrExt{
    fn is_serde_attr(&self) -> bool;
    fn is_serde_default(&self) -> bool;
}

impl IAttrExt for syn::Attribute{
    fn is_serde_attr(&self) -> bool {
        let Some(last_segment) = self.path().segments.last() else {
            return false;
        };
        let expected: syn::PathSegment = parse_quote!(serde);
        return *last_segment == expected
    }
    fn is_serde_default(&self) -> bool {
        if !self.is_serde_attr() {
            return false
        }
        if matches!(self.style, syn::AttrStyle::Inner(_)){
            return false;
        }
        let syn::Meta::List(meta_list) = &self.meta else {
            return false;
        };
        match meta_list.parse_args::<SerdeDefaultAttrParams>() {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

pub trait IVariantExt {
    fn partial_field_name(&self) -> syn::Ident;
    fn as_partial_field(&self) -> syn::Result<syn::Field>;
    fn tag(&self, outer_rename: Option<&SerdeOuterRenameParams>) -> syn::LitStr;
    fn field_types(&self) -> Vec<syn::Type>;
}

impl IVariantExt for syn::Variant {
    fn partial_field_name(&self) -> syn::Ident{
        let ident = heck::AsSnakeCase(self.ident.to_string()).to_string();
        syn::Ident::new(&ident, self.ident.span())
    }
    
    fn as_partial_field(&self) -> syn::Result<syn::Field> {
        let unnamed_fields = match &self.fields{
            syn::Fields::Unnamed(syn::FieldsUnnamed{unnamed, ..}) => {
                unnamed
            },
            _ => return Err(syn::Error::new(self.span(), "Only unnamed fields supported for now"))
        };
        let ident = self.partial_field_name();
        let field_type: syn::Type = parse_quote!{
            Option<  <#unnamed_fields as ::bioimg_spec::util::AsPartial>::Partial  >
        };
        Ok(parse_quote!(#ident : #field_type))
    }

    fn tag(&self, outer_rename: Option<&SerdeOuterRenameParams>) -> syn::LitStr {
        self.attrs.iter()
            .find_map(|attr| SerdeInnerRenameParams::try_from_attr(attr))
            .map(|params| params.new_name)
            .unwrap_or_else(||{
                let default_tag = syn::LitStr::new(&self.ident.to_string(), self.ident.span());
                match outer_rename {
                    Some(rename) => rename.rename_style.transform(&default_tag),
                    None => default_tag
                }
            })
    }
    fn field_types(&self) -> Vec<syn::Type>{
        match &self.fields{
            syn::Fields::Unnamed(unnamed_fields) => unnamed_fields.unnamed.iter().map(|f| f.ty.clone()).collect(),
            syn::Fields::Named(named_fields) => named_fields.named.iter().map(|f| f.ty.clone()).collect(),
            syn::Fields::Unit => vec![]
        }
    }
}

pub trait IEnumExt {
    // fn partial_fields(&self) -> impl Iterator<Item=syn::Field>;
    fn tagged_variants(&self) -> impl Iterator<Item=(syn::LitStr, &syn::Variant)>;
    fn as_partial_struct(&self) -> (syn::ItemStruct, syn::ItemImpl);
}

impl IEnumExt for syn::ItemEnum {
    fn tagged_variants(&self) -> impl Iterator<Item=(syn::LitStr, &syn::Variant)> {
        let rename_params = self.attrs.iter().find_map(|attr| SerdeOuterRenameParams::try_from_attr(attr));
        self.variants.iter().map(move |v| (v.tag(rename_params.as_ref()), v) )
    }
    fn as_partial_struct(&self) -> (syn::ItemStruct, syn::ItemImpl) {
        let tag_params = self.attrs.iter().find_map(SerdeEnumTagParams::try_from_attr);

        let global_rename_params = SerdeEnumTagParams::from_attributes(&self.attrs);

        let (partial_struct_field_idents, variant_tags): (Vec<syn::Ident>, Vec<syn::LitStr>) = self.tagged_variants()
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

        let partial_struct_ident = format_ident!("Partial{}", self.ident);
        let partial_struct_fields: Vec<syn::Field> = self.variants.iter()
            .map(|v| v.as_partial_field())
            .collect::<syn::Result<_>>()?;
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();

        let partial_struct: syn::ItemStruct = parse_quote!{
            #[derive(Clone, Debug, ::serde::Deserialize)]
            #[serde(try_from = "::serde_json::Value")]
            pub struct #partial_struct_ident #impl_generics
            #where_clause
            {
                #(#partial_struct_fields),*
            }
        };
        let impl_TryFrom: syn::ItemImpl = parse_quote!{
            impl<#impl_generics> TryFrom<::serde_json::Value> for $partial_struct_ident {
                #fn_tryFrom
            }
        };

        (partial_struct, impl_TryFrom)
    }
}

pub trait IStructExt{
    fn field_types(&self) -> Vec<syn::Type>;
}

impl IStructExt for syn::ItemStruct {
    fn field_types(&self) -> Vec<syn::Type>{
        self.fields.iter().map(|f| f.ty.clone()).collect()
    }
}

pub trait IWhereClauseExt{
    fn for_partial(input: &syn::DeriveInput) -> syn::WhereClause;
}

impl IWhereClauseExt for syn::WhereClause {
    fn for_partial(input: &syn::DeriveInput) -> syn::WhereClause {
        let mut wc = input.generics.where_clause.clone().unwrap_or(parse_quote!(where));

        let comma = parse_quote!(,);
        if !wc.predicates.empty_or_trailing() {
            wc.predicates.push_punct(comma);
        }

        let field_types = match &input.data {
            syn::Data::Struct(st) => st.fields.iter().map(|f| f.ty.clone()).collect(),
            syn::Data::Enum(enm) => enm.variants.iter()
                .map(|v| v.field_types().into_iter())
                .flatten()
                .collect(),
            syn::Data::Union(_) => vec![],
        };

        for field_ty in &field_types {
            let span = field_ty.span();
            wc.predicates.push_value(parse_quote_spanned!{ span=>
                #field_ty: ::aspartial::util::AsSerializablePartial<Partial: std::clone::Clone + std::fmt::Debug>
            });
            wc.predicates.push_punct(comma);
        }
        wc
    }
}

// pub trait IFieldExt{
//     fn to_partial_field()
// }


// use quote::quote;

// pub trait AttributeExt {
// }

// impl AttributeExt for syn::Attribute {
// }


// pub trait FieldExt {
// }


// pub trait IdentExt {
//     fn to_lit_str(&self) -> syn::LitStr;
// }

// impl IdentExt for syn::Ident {
//     fn to_lit_str(&self) -> syn::LitStr {
//         syn::LitStr::new(&quote!(#self).to_string(), self.span())
//     }
// }
// 
