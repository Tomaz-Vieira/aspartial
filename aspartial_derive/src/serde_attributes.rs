use crate::{syn_extensions::IAttrExt, util::KeyEqualsLitStr};


pub struct SerdeDefaultAttrParams;

impl syn::parse::Parse for SerdeDefaultAttrParams {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let first_token_span = input.span();
        let default_token: syn::Ident = input.parse()?;
        if default_token.to_string() != "default" {
            return Err(syn::Error::new(first_token_span, "Expected 'default' token"))
        }
        if input.is_empty() {
            return Ok(Self)
        }
        input.parse::<syn::Token![=]>()?;
        let _default_function_name: syn::LitStr = input.parse()?;
        Ok(Self)
    }
}

pub enum SerdeEnumTagParams{
    Untagged,
    InternallyTagged{tag_key: syn::LitStr},
    AdjacentlyTagged{tag_key: syn::LitStr, content_key: syn::LitStr},
    ExternallyTagged,
}

impl SerdeEnumTagParams {
    pub fn try_from_attr(attr: &syn::Attribute) -> Option<Self>{
        if !attr.is_serde_attr(){
            return None
        }
        let syn::Meta::List(meta_list) = &attr.meta else {
            return None
        };
        meta_list.parse_args::<Self>().ok()
    }

    pub fn from_attributes(attributes: &[syn::Attribute]) -> Self{
        for attr in attributes{
            if let Some(params) = Self::try_from_attr(attr) {
                return params
            }
        }
        return Self::ExternallyTagged
    }
}

impl syn::parse::Parse for SerdeEnumTagParams {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if let Ok(ident) = input.fork().parse::<syn::Ident>(){
            if ident.to_string().as_str() == "untagged" {
                return Ok(Self::Untagged)
            }
        }

        let params_span = input.span();
        let mut key_vals = input.parse_terminated(KeyEqualsLitStr::parse, syn::Token![,])?.into_iter();
        let Some(key_val1) = key_vals.next() else {
            return Err(syn::Error::new(params_span, "Expected at least one param"))
        };
        let Some(key_val2) = key_vals.next() else {
            if key_val1.key != "tag" {
                return Err(syn::Error::new(key_val1.key.span(), "expected key to be 'tag'"))
            }
            return Ok(Self::InternallyTagged { tag_key: key_val1.value })
        };
        if let Some(extra_keyval) = key_vals.next() {
            return Err(syn::Error::new(extra_keyval.key.span(), "Expected at most one param"))
        }

        let (content_keyval, tag_keyval) = if key_val1.key.to_string() < key_val2.key.to_string() {
            (key_val1, key_val2)
        }else{
            (key_val2, key_val1)
        };

        if content_keyval.key.to_string() != "content"{
            return Err(syn::Error::new(content_keyval.key.span(), "expected key to be 'content'"))
        }
        if tag_keyval.key.to_string() != "tag"{
            return Err(syn::Error::new(tag_keyval.key.span(), "expected key to be 'tag'"))
        }

        Ok(Self::AdjacentlyTagged { tag_key: tag_keyval.value, content_key: content_keyval.value })
    }
}

pub struct SerdeInnerRenameParams{
    /// `rename` in #[serde(rename = "bla")]
    #[allow(dead_code)]
    pub rename_marker: syn::Ident,
    #[allow(dead_code)]
    pub equals_sign: syn::Token![=],
    /// `"bla"` in #[serde(rename = "bla")]
    pub new_name: syn::LitStr,
}

impl syn::parse::Parse for SerdeInnerRenameParams {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key_val: KeyEqualsLitStr = input.parse()?;
        if key_val.key.to_string() != "rename" {
            return Err(syn::Error::new(key_val.key.span(), "Expected key 'rename'"))
        }
        Ok(Self{
            rename_marker: key_val.key,
            equals_sign: key_val.equals_token,
            new_name: key_val.value
        })
    }
}

impl SerdeInnerRenameParams {
    pub fn try_from_attr(attr: &syn::Attribute) -> Option<Self> {
        if !attr.is_serde_attr(){
            return None
        }
        let syn::Meta::List(meta_list) = &attr.meta else {
            return None
        };
        meta_list.parse_args::<Self>().ok()
    }
}

pub enum RenameStyle{
    Lowercase,
    Uppercase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

impl RenameStyle {
    pub fn transform(&self, litstr: &syn::LitStr) -> syn::LitStr {
        let raw = litstr.value();
        let transformed_raw = match self {
            Self::Lowercase => raw.to_lowercase(),
            Self::Uppercase => raw.to_uppercase(),
            Self::PascalCase => heck::AsPascalCase(raw).to_string(),
            Self::CamelCase => heck::AsLowerCamelCase(raw).to_string(),
            Self::SnakeCase => heck::AsSnakeCase(raw).to_string(),
            Self::ScreamingSnakeCase => heck::AsShoutySnakeCase(raw).to_string(),
            Self::KebabCase => heck::AsKebabCase(raw).to_string(),
            Self::ScreamingKebabCase => heck::AsShoutyKebabCase(raw).to_string(),
        };
        syn::LitStr::new(&transformed_raw, litstr.span())
    }
}

impl TryFrom<&syn::LitStr> for RenameStyle {
    type Error = syn::Error;

    fn try_from(litstr: &syn::LitStr) -> Result<Self, Self::Error> {
        let raw = litstr.value();
        Ok(match raw.as_str() {
            "lowercase" => Self::Lowercase,
            "UPPERCASE" => Self::Uppercase,
            "PascalCase" => Self::PascalCase,
            "camelCase" => Self::CamelCase,
            "snake_case" => Self::SnakeCase,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnakeCase,
            "kebab-case" => Self::KebabCase,
            "SCREAMING-KEBAB-CASE" => Self::ScreamingKebabCase,
            _ => return Err(syn::Error::new(litstr.span(), "Invalid rename style"))
        })
    }
}

pub struct SerdeOuterRenameParams {
    /// `rename_all` in #[serde(rename_all = "bla")]
    #[allow(dead_code)]
    pub rename_marker: syn::Ident,
    #[allow(dead_code)]
    pub equals_sign: syn::Token![=],
    pub rename_style: RenameStyle,
}

impl syn::parse::Parse for SerdeOuterRenameParams {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key_val: KeyEqualsLitStr = input.parse()?;
        if key_val.key.to_string() != "rename_all" {
            return Err(syn::Error::new(key_val.key.span(), "Expected key 'rename_all'"))
        }
        let rename_style = RenameStyle::try_from(&key_val.value)?;
        Ok(Self{
            rename_marker: key_val.key,
            equals_sign: key_val.equals_token,
            rename_style,
        })
    }
}

impl SerdeOuterRenameParams {
    pub fn try_from_attr(attr: &syn::Attribute) -> Option<Self> {
        if !attr.is_serde_attr(){
            return None
        }
        let syn::Meta::List(meta_list) = &attr.meta else {
            return None
        };
        meta_list.parse_args::<Self>().ok()        
    }
}
