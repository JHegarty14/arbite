use proc_macro2::Span;
use syn::{AttributeArgs, NestedMeta};

pub(crate) struct Args {
    pub(crate) path: syn::LitStr,
    pub(crate) methods: Vec<syn::Path>,
    pub(crate) wrappers: Vec<syn::Path>,
}

impl Args {
    pub(crate) fn new(args: AttributeArgs) -> syn::Result<Self> {
        println!("Args::new({:?})", args);
        let mut path = None;
        let mut methods = Vec::new();
        let mut wrappers = Vec::new();
        for arg in args {
            match arg {
                NestedMeta::Lit(syn::Lit::Str(lit)) => match path {
                    None => {
                        path = Some(lit);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            lit,
                            "Multiple paths provided! Only one path can be specified",
                        ));
                    }
                },
                NestedMeta::Meta(syn::Meta::List(nv)) => {
                    if nv.path.is_ident("method") {
                        for method in nv.nested {
                            if let syn::NestedMeta::Meta(syn::Meta::Path(path)) = method {
                                methods.push(path);
                            } else {
                                return Err(syn::Error::new_spanned(
                                    nv.path,
                                    "Attribute method expects a path!",
                                ));
                            }
                        }
                    } else if nv.path.is_ident("wrap") {
                        for wrapper in nv.nested {
                            if let syn::NestedMeta::Meta(syn::Meta::Path(path)) = wrapper {
                                wrappers.push(path);
                            } else {
                                return Err(syn::Error::new_spanned(
                                    nv.path,
                                    "Attribute wrap expects type",
                                ));
                            }
                        }
                    }
                }
                NestedMeta::Meta(syn::Meta::NameValue(nv)) => {
                    if nv.path.is_ident("path") {
                        if let syn::Lit::Str(lit) = nv.lit {
                            path = Some(lit);
                        } else {
                            return Err(syn::Error::new_spanned(
                                nv.lit,
                                "Path expects literal string.",
                            ));
                        }
                    } else {
                        return Err(syn::Error::new_spanned(
                            nv.path,
                            "Unknown attribute key is specified.",
                        ));
                    }
                }
                arg => {
                    return Err(syn::Error::new_spanned(arg, "Unknown attribute."));
                }
            }
        }
        Ok(Args {
            path: path.unwrap_or_else(|| syn::LitStr::new("/", proc_macro2::Span::call_site())),
            methods,
            wrappers,
        })
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            path: syn::LitStr::new("", Span::call_site()),
            methods: Vec::new(),
            wrappers: Vec::new(),
        }
    }
}
