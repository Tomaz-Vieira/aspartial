use syn::{parse_quote, spanned::Spanned};

use crate::{serde_attributes::{SerdeInnerRenameParams, SerdeOuterRenameParams}, util::KeyEqualsLitStr};

pub trait IAttrExt{
    fn is_serde_attr(&self) -> bool;
    fn is_serde_any_default(&self) -> bool;
    fn is_serde_regular_default(&self) -> bool;
    fn as_serde_default_func_path(&self) -> Option<syn::Path>;
    fn is_serde_default_to_func(&self) -> bool {
        self.as_serde_default_func_path().is_some()
    }
}

pub trait IFieldExt {
    fn partial_is_optional(&self) -> bool;
    fn is_serde_default(&self) -> bool;
    fn partial_type(&self) -> syn::Type;
}
impl IFieldExt for syn::Field{
    fn partial_is_optional(&self) -> bool{
        !self.is_serde_default()
    }
    fn is_serde_default(&self) -> bool{
        self.attrs.iter().any(|attr| attr.is_serde_any_default())
    }
    fn partial_type(&self) -> syn::Type {
        let field_ty = &self.ty;
        let partial_type: syn:: Type = parse_quote!(<#field_ty as ::aspartial::AsPartial>::Partial);

        if self.is_serde_default() {
            partial_type
        } else {
            parse_quote!(Option< #partial_type >)
        }
    }
}

impl IAttrExt for syn::Attribute{
    fn is_serde_attr(&self) -> bool {
        let Some(last_segment) = self.path().segments.last() else {
            return false;
        };
        return last_segment.ident.to_string() == "serde"
    }
    fn is_serde_any_default(&self) ->bool {
        self.is_serde_regular_default() || self.is_serde_default_to_func()
    }
    fn is_serde_regular_default(&self) -> bool {
        if !self.is_serde_attr() {
            return false
        }
        if matches!(self.style, syn::AttrStyle::Inner(_)){
            return false;
        }
        let syn::Meta::List(meta_list) = &self.meta else {
            return false;
        };
        let Ok(default_token) = meta_list.parse_args::<syn::Ident>() else {
            return false
        };
        return default_token.to_string() == "default"
    }
    fn as_serde_default_func_path(&self) -> Option<syn::Path> {
        if !self.is_serde_attr() {
            return None
        }
        if matches!(self.style, syn::AttrStyle::Inner(_)){
            return None;
        }
        let syn::Meta::List(meta_list) = &self.meta else {
            return None;
        };
        let Ok(keyval) = meta_list.parse_args::<KeyEqualsLitStr>() else {
            return None
        };
        if keyval.key.to_string() != "default" {
            return None
        }
        keyval.value.parse().ok()
    }
}

pub trait IVariantExt {
    fn partial_field_name(&self) -> syn::Ident;
    fn as_partial_field(&self) -> syn::Result<syn::Field>;
    fn tag(&self, outer_rename: Option<&SerdeOuterRenameParams>) -> syn::LitStr;
    fn fields(&self) -> impl Iterator<Item=&syn::Field>;
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
            Option<  <#unnamed_fields as ::aspartial::AsPartial>::Partial  >
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
    fn fields(&self) -> impl Iterator<Item=&syn::Field>{
        let out: Box<dyn Iterator<Item=_>> = match &self.fields{
            syn::Fields::Unnamed(unnamed_fields) => Box::new(unnamed_fields.unnamed.iter()),
            syn::Fields::Named(named_fields) => Box::new(named_fields.named.iter()),
            syn::Fields::Unit => Box::new(std::iter::empty()),
        };
        out
    }
}

pub trait IEnumExt {
    // fn partial_fields(&self) -> impl Iterator<Item=syn::Field>;
    fn tagged_variants(&self) -> impl Iterator<Item=(syn::LitStr, &syn::Variant)>;
}

impl IEnumExt for syn::ItemEnum {
    fn tagged_variants(&self) -> impl Iterator<Item=(syn::LitStr, &syn::Variant)> {
        let rename_params = self.attrs.iter().find_map(|attr| SerdeOuterRenameParams::try_from_attr(attr));
        self.variants.iter().map(move |v| (v.tag(rename_params.as_ref()), v) )
    }
}
