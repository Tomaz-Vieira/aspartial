use syn::{parse_quote, spanned::Spanned};

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
    fn as_partial_field(&self) -> syn::Result<syn::Field>;
    fn serialized_field_name(&self) -> syn::LitStr;
}

impl IVariantExt for syn::Variant {
    fn as_partial_field(&self) -> syn::Result<syn::Field> {
        let ident = {
            let ident = heck::AsSnakeCase(self.ident.to_string()).to_string();
            syn::Ident::new(&ident, self.ident.span())
        };
        let unnamed_fields = match &self.fields{
            syn::Fields::Unnamed(syn::FieldsUnnamed{unnamed, ..}) => {
                unnamed
            },
            _ => return Err(syn::Error::new(self.span(), "Only unnamed fields supported for now"))
        };
        let field_type: syn::Type = parse_quote!{
            Option<  <#unnamed_fields as ::bioimg_spec::util::AsPartial>::Partial  >
        };
        Ok(parse_quote!(#ident : #field_type))
    }

    fn serialized_field_name(&self) -> syn::LitStr {
        self.attrs.iter()
            .find_map(|attr| SerdeInnerRenameParams::try_from_attr(attr))
            .map(|params| params.new_name)
            .unwrap_or_else(||{
                let ident = heck::AsSnakeCase(self.ident.to_string()).to_string();
                syn::LitStr::new(&ident, self.ident.span())
            })
    }
}

pub trait IEnumExt {
    fn partial_fields(&self) -> impl Iterator<Item=syn::Field>;
}

impl IEnumExt for syn::ItemEnum {
    fn partial_fields(&self) -> Vec<syn::Field> {
        match self.attrs.iter().find(|attr| SerdeOuterRenameParams::try_from_attr(attr)) => {
            None => self.variants.iter()
                .map(|v| v.as_partial_field())
                .collect(),
            Some(rename) => self.variants.iter()
                .map(|v| {
                    let field = v.as_partial_field();
                })
        }
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
