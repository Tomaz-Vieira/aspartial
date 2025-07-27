use proc_macro2::Span;

pub struct NameConfig {
    pub partial_type_key: syn::Ident,
    #[allow(dead_code)]
    pub equals_sign: syn::Token![=],
    pub ident: syn::Ident,
}

pub struct PartialIsInnerConfig{
    pub partial_is_inner_keyword: syn::Ident,
}

pub struct AttrsConfig {
    #[allow(dead_code)]
    pub attrs_key: syn::Ident,
    #[allow(dead_code)]
    pub opening_paren: syn::token::Paren,
    pub attrs: Vec<syn::Attribute>
}

pub enum ModeConfig {
    /// Determines the name of the generated partial type
    Name(NameConfig),
    /// Use inner type in a newtype-like struct as the partial type
    PartialIsInner(PartialIsInnerConfig),
}

impl From<NameConfig> for ModeConfig {
    fn from(value: NameConfig) -> Self {
        Self::Name(value)
    }
}

impl From<PartialIsInnerConfig> for ModeConfig {
    fn from(value: PartialIsInnerConfig) -> Self {
        Self::PartialIsInner(value)
    }
}

//////////////////////////////

pub enum Config{
    /// Determines the name of the generated partial type
    Name(NameConfig),
    /// Use inner type in a newtype-like struct as the partial type
    PartialIsInner(PartialIsInnerConfig),
    /// Add the attributes to the generated type
    Attrs(AttrsConfig),
}

impl From<ModeConfig> for Config {
    fn from(value: ModeConfig) -> Self {
        match value{
            ModeConfig::Name(conf) => Self::Name(conf),
            ModeConfig::PartialIsInner(conf) => Self::PartialIsInner(conf),
        }
    }
}
impl From<NameConfig> for Config {
    fn from(value: NameConfig) -> Self {
        Self::Name(value)
    }
}
impl From<PartialIsInnerConfig> for Config {
    fn from(value: PartialIsInnerConfig) -> Self {
        Self::PartialIsInner(value)
    }
}
impl From<AttrsConfig> for Config {
    fn from(value: AttrsConfig) -> Self {
        Self::Attrs(value)
    }
}

///////////////////////////////

impl syn::parse::Parse for Config {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        match ident.to_string().as_str() {
            "name" =>  Ok(NameConfig {
                partial_type_key: ident,
                equals_sign: input.parse()?,
                ident: input.parse()?,
            }.into()),
            "attrs" => {
                let attrs_content;
                return Ok(AttrsConfig{
                    attrs_key: ident,
                    opening_paren: syn::parenthesized!(attrs_content in input),
                    attrs: attrs_content.call(syn::Attribute::parse_outer)?,
                }.into())
            },
            "newtype" => Ok(PartialIsInnerConfig{partial_is_inner_keyword: ident}.into()),
            _ => Err(syn::Error::new(
                ident.span(),
                format!("Unrecognized AsPartial config. Expected 'name', 'newtype' or 'attrs', found '{ident}'")
            ))
        }
    }
}

///////////////////////////////

pub struct ConfigsForAsPartial {
    pub mode: ModeConfig,
    pub attrs: Vec<syn::Attribute>,
}

impl ConfigsForAsPartial {
    pub fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut mode: syn::Result<ModeConfig> = Err(
            syn::Error::new(Span::call_site(), "must specify name or newtype mode")
        );
        let mut attrs_for_partial_config = Vec::<syn::Attribute>::new();

        for attr in attrs {
            if attr.path().segments.last().unwrap().ident.to_string() != "aspartial" {
                continue
            }
            let syn::Meta::List(meta_list) = &attr.meta else {
                continue
            };
            match meta_list.parse_args::<Config>()? {
                Config::Name(conf) => {
                    let span = conf.partial_type_key.span();
                    if let Ok(_) = std::mem::replace(&mut mode, Ok(conf.into())) {
                        return Err(syn::Error::new(span, "Setting mode again"))
                    }
                },
                Config::PartialIsInner(conf) => {
                    let span = conf.partial_is_inner_keyword.span();
                    if let Ok(_) = std::mem::replace(&mut mode, Ok(conf.into())) {
                        return Err(syn::Error::new(span, "Setting mode again"))
                    }
                },
                Config::Attrs(new_attrs_conf) => {
                    attrs_for_partial_config.extend(new_attrs_conf.attrs);
                },
            }
        }

        Ok(Self{
            mode: mode?,
            attrs: attrs_for_partial_config,
        })
    }
}


