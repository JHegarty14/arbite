use proc_macro2::{Ident, Span};
use std::collections::HashMap;
use syn::Meta;

pub(crate) struct ClientArgs {
    pub(crate) interceptors: Vec<syn::Path>,
}

impl ClientArgs {
    pub(crate) fn parse_and_strip(attrs: &mut Vec<syn::Attribute>) -> syn::Result<Self> {
        let mut interceptors = Vec::new();
        let mut path_to_vec = HashMap::new();
        let call_site = Span::call_site();
        path_to_vec.insert(Ident::new("interceptors", call_site), &mut interceptors);
        for attr in attrs.clone() {
            match attr.parse_meta() {
                Ok(Meta::List(nv)) => {
                    if let Some(vec) = nv.path.get_ident().and_then(|x| path_to_vec.get_mut(x)) {
                        for item in nv.nested {
                            if let syn::NestedMeta::Meta(syn::Meta::Path(path)) = item {
                                vec.push(path);
                            } else {
                                return Err(syn::Error::new_spanned(
                                    nv.path,
                                    "Expected a function name!",
                                ));
                            }
                        }
                    } else {
                        return Err(syn::Error::new_spanned(
                            nv.path,
                            "Unknown attribute key is specified.",
                        ));
                    }
                }
                Ok(Meta::Path(path)) => {
                    return Err(syn::Error::new_spanned(
                        path,
                        "Unknown attribute key is specified.",
                    ))
                }
                Ok(arg) => {
                    return Err(syn::Error::new_spanned(arg, "Unknown attribute."));
                }
                Err(_) => {}
            }
        }
        attrs.retain(|attr| {
            attr.path
                .get_ident()
                .and_then(|x| path_to_vec.get(x))
                .is_none()
        });
        Ok(Self { interceptors })
    }
}