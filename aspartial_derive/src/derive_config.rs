use proc_macro2::Span;
use syn::parse_quote;

struct NameConfig {
    partial_type_key: syn::Ident,
    equals_sign: syn::Token![=],
    ident: syn::Ident,
}

pub struct AttrsConfig {
    pub attrs_key: syn::Ident,
    pub opening_paren: syn::token::Paren,
    pub attrs: Vec<syn::Attribute>
}

pub enum Config{
    Name(NameConfig),
    Attrs(AttrsConfig),
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
        Err(syn::Error::new(ident.span(), "Unrecognized AsPartial config. Expected 'name' or 'attrs'"))
    }
}

pub struct ConfigsForAsPartial {
    pub partial_type_ident: syn::Ident,
    pub attrs: Vec<syn::Attribute>,
}

impl ConfigsForAsPartial {
    pub fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut name_config: syn::Result<NameConfig> = Err(syn::Error::new(Span::call_site(), "no partial name set"));
        let mut attrs_for_partial_config = Vec::<syn::Attribute>::new();

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
                    continue
                },
                Config::Attrs(new_attrs_conf) => {
                    attrs_for_partial_config.extend(new_attrs_conf.attrs);
                },
            }
        }

        Ok(Self{
            partial_type_ident: name_config?.ident,
            attrs: attrs_for_partial_config,
        })
    }
}


