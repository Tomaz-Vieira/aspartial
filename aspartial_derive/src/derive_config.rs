use proc_macro2::Span;
use syn::spanned::Spanned;
use quote::quote;

use crate::syn_extensions::IAttrExt;

pub struct NameConfig {
    pub partial_type_key: syn::Ident,
    #[allow(dead_code)]
    pub equals_sign: syn::Token![=],
    pub ident: syn::Ident,
}

pub struct AttrsConfig {
    #[allow(dead_code)]
    pub attrs_key: syn::Ident,
    #[allow(dead_code)]
    pub opening_paren: syn::token::Paren,
    pub attrs: Vec<syn::Attribute>
}

pub struct DeriveTryFromJsonValueConfig{
    pub derive_key: syn::Ident,
    #[allow(dead_code)]
    pub opening_paren: syn::token::Paren,
    #[allow(dead_code)]
    pub derive_path: syn::Path,
}

pub enum Config{
    Name(NameConfig),
    Attrs(AttrsConfig),
    DeriveFromJsonValue(DeriveTryFromJsonValueConfig),
}

impl syn::parse::Parse for Config {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        if ident.to_string() == "name" {
            return Ok(Self::Name(NameConfig {
                partial_type_key: ident,
                equals_sign: input.parse()?,
                ident: input.parse()?,
            }))
        }
        if ident.to_string() == "attrs" {
            let attrs_content;
            return Ok(Self::Attrs(AttrsConfig{
                attrs_key: ident,
                opening_paren: syn::parenthesized!(attrs_content in input),
                attrs: attrs_content.call(syn::Attribute::parse_outer)?,
            }))
        }

        if ident.to_string() == "derive"{
            if ! cfg!(feature="serde_json") {
                return Err(syn::Error::new(ident.span(), "'derive' only allowed with 'serde_json' feature enabled"))
            }
            let derive_content;
            let opening_paren = syn::parenthesized!(derive_content in input);
            let derive_path: syn::Path = derive_content.parse()?;
            if quote!(#derive_path).to_string() != quote!(TryFrom<::serde_json::Value>).to_string() {
                return Err(syn::Error::new(derive_path.span(), "Expected derive to be 'TryFrom<::serde_json::Value>'"));
            }
            return Ok(Self::DeriveFromJsonValue(DeriveTryFromJsonValueConfig{
                derive_key: ident,
                opening_paren,
                derive_path,
            }))
        }
        Err(syn::Error::new(
            ident.span(),
            format!("Unrecognized AsPartial config. Expected 'name', 'attrs' or 'derive', found {ident}")
        ))
    }
}

pub struct ConfigsForAsPartial {
    pub partial_type_ident: syn::Ident,
    pub attrs: Vec<syn::Attribute>,
    pub derive_from_json_value: Option<DeriveTryFromJsonValueConfig>,
}

impl ConfigsForAsPartial {
    pub fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut name_config: syn::Result<NameConfig> = Err(syn::Error::new(Span::call_site(), "no partial name set"));
        let mut attrs_for_partial_config = Vec::<syn::Attribute>::new();
        let mut derive__try_from__json_val: Option<DeriveTryFromJsonValueConfig> = None;

        for attr in attrs {
            if attr.path().segments.last().unwrap().ident.to_string() != "aspartial" {
                continue
            }
            let syn::Meta::List(meta_list) = &attr.meta else {
                continue
            };
            match meta_list.parse_args::<Config>()? {
                Config::Name(new_name_conf) => {
                    if name_config.is_ok() {
                        return Err(syn::Error::new(new_name_conf.partial_type_key.span(), "Setting partial name again"))
                    }
                    name_config = Ok(new_name_conf);
                },
                Config::Attrs(new_attrs_conf) => {
                    attrs_for_partial_config.extend(new_attrs_conf.attrs);
                },
                Config::DeriveFromJsonValue(new_from_json_val_config) => {
                    if derive__try_from__json_val.is_some(){
                        return Err(syn::Error::new(new_from_json_val_config.derive_key.span(), "Setting derive again"))
                    }
                    derive__try_from__json_val.replace(new_from_json_val_config);
                },
            }
        }

        Ok(Self{
            partial_type_ident: name_config?.ident,
            attrs: attrs_for_partial_config,
            derive_from_json_value: derive__try_from__json_val,
        })
    }
}


