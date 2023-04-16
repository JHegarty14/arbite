use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, TokenStreamExt};
use std::str::FromStr;
use syn::Ident;

#[derive(PartialEq)]
pub(crate) enum MethodType {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Connect,
    Options,
    Trace,
    Patch,
}

impl MethodType {
    fn as_guard(&self) -> &'static str {
        match self {
            MethodType::Get => "Get",
            MethodType::Post => "Post",
            MethodType::Put => "Put",
            MethodType::Delete => "Delete",
            MethodType::Head => "Head",
            MethodType::Connect => "Connect",
            MethodType::Options => "Options",
            MethodType::Trace => "Trace",
            MethodType::Patch => "Patch",
        }
    }
}

impl FromStr for MethodType {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "get" => Ok(MethodType::Get),
            "post" => Ok(MethodType::Post),
            "put" => Ok(MethodType::Put),
            "delete" => Ok(MethodType::Delete),
            "head" => Ok(MethodType::Head),
            "connect" => Ok(MethodType::Connect),
            "options" => Ok(MethodType::Options),
            "trace" => Ok(MethodType::Trace),
            "patch" => Ok(MethodType::Patch),
            _ => Err(()),
        }
    }
}

impl ToTokens for MethodType {
    fn to_tokens(&self, stream: &mut TokenStream2) {
        let ident = Ident::new(self.as_guard(), Span::call_site());
        stream.append(ident);
    }
}
