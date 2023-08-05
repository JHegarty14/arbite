use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, Ident, ItemImpl, ItemStruct};
mod args;
mod client;
mod injected;
mod module;
mod route;
use crate::client::ClientArgs;
use crate::injected::InjectedBody;
use crate::module::ModuleArgs;
use crate::route::MethodType;
use args::Args;
use std::str::FromStr;

/// Macro to denote async application entry point
#[proc_macro_attribute]
pub fn go(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = syn::parse_macro_input!(item as syn::ItemFn);
    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &mut input.sig;
    let body = &input.block;

    if sig.asyncness.is_none() {
        return syn::Error::new_spanned(sig.fn_token, "function must be async!")
            .to_compile_error()
            .into();
    }

    sig.asyncness = None;

    (quote! {
        #(#attrs)*
        #vis #sig {
            sept::Runtime::new()
                .block_on(async move { #body })
        }
    })
    .into()
}

#[proc_macro_attribute]
pub fn go_test(_: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let body = &input.block;
    let attrs = &input.attrs;
    let mut has_test_attr = false;

    for attr in attrs {
        if attr.path.is_ident("test") {
            has_test_attr = true;
        }
    }

    if input.sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            input.sig.fn_token,
            format!("function must be async!, {}", input.sig.ident),
        )
        .to_compile_error()
        .into();
    }

    let result = if has_test_attr {
        quote! {
            #(#attrs)*
            fn #name() #ret {
                sept::Runtime::new("test")
                    .block_on(async { #body })
            }
        }
    } else {
        quote! {
            #[test]
            #(#attrs)*
            fn #name() #ret {
                sept::Runtime::new("test")
                    .block_on(async { #body })
            }
        }
    };

    result.into()
}

/// Derives the `Injectable` trait for dependency injection.
#[proc_macro_derive(Injectable)]
pub fn injectable(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let name = &ast.ident;
    let graph_ident = Ident::new("graph", Span::call_site());
    let context_ident = Ident::new("ctx", Span::call_site());
    let fields = match &ast.data {
        syn::Data::Struct(st) => match InjectedBody::new(&graph_ident, &context_ident, st) {
            Ok(fields) => Ok(fields),
            err => err,
        },
        _ => Err(syn::Error::new_spanned(
            &ast,
            "Can only be applied to structs",
        )),
    };
    match fields {
        Ok(f) => {
            let expanded = quote! {
                #[automatically_derived]
                impl sept::graph::Injected for #name {
                    type Output = Self;
                    fn resolve(
                        #graph_ident: &mut sept::graph::Graph,
                        #context_ident: &[&sept::graph::Graph]
                    ) -> Self {
                        Self {
                            #f
                        }
                    }
                }
            };
            TokenStream::from(expanded)
        }
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn module(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    match ModuleArgs::parse_and_strip(&mut input.attrs) {
        Ok(ModuleArgs {
            clients,
            imports,
            exports,
            providers,
        }) => {
            let expanded = quote! {
                #input

                #[automatically_derived]
                impl sept::sept_module::ModuleFactory for #name {
                    fn get_module() -> sept::sept_module::Module {
                        sept::sept_module::Module::new()
                            #(.import::<#imports>())*
                            #(.export::<#exports>())*
                            #(.provide::<#providers>())*
                            #(.client::<#clients>())*
                    }
                }
            };
            TokenStream::from(expanded)
        }
        Err(err) => err.to_compile_error().into(),
    }
}

struct Method {
    name: Ident,
    method_type: MethodType,
    args: Args,
    impl_item: syn::ImplItemMethod,
}

impl Method {
    fn new(impl_item: &mut syn::ImplItemMethod) -> Result<Option<Self>, syn::Error> {
        let mut method_type = None;
        let mut args = None;
        let mut err = None;
        impl_item.attrs.retain(|attr| {
            match attr.parse_meta() {
                Ok(syn::Meta::List(list)) => {
                    if let Some(ident) = list.path.get_ident() {
                        if let Ok(mt) = MethodType::from_str(&*ident.to_string()) {
                            method_type = Some(mt);
                            match Args::new(list.nested.into_iter().collect()) {
                                Ok(ar) => {
                                    args = Some(ar);
                                }
                                Err(e) => err = Some(e),
                            }
                            return false;
                        }
                    }
                }
                Ok(syn::Meta::Path(path)) => {
                    if let Some(ident) = path.get_ident() {
                        if let Ok(mt) = MethodType::from_str(&*ident.to_string()) {
                            method_type = Some(mt);
                            return false;
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => {}
            }
            true
        });

        if let Some(err_inner) = err {
            return Err(err_inner);
        }

        match method_type {
            Some(mt) => Ok(Some(Self {
                name: format_ident!("_{}_{}_", "sept", impl_item.sig.ident),
                method_type: mt,
                args: args.unwrap_or_default(),
                impl_item: impl_item.clone(),
            })),
            None => Ok(None),
        }
    }
}

impl ToTokens for Method {
    fn to_tokens(&self, stream: &mut TokenStream2) {
        let Self {
            name,
            method_type,
            args:
                Args {
                    path,
                    methods,
                    wrappers,
                },
            impl_item,
        } = self;
        let target = &impl_item.sig.ident;
        let expanded = quote! {
            #[allow(non_snake_case)]
            fn #name(&self) -> actix_web::Resource {
                actix_web::web::resource(#path)
                    .guard(actix_web::guard::#method_type())
                    #(.guard(actix_web::guard::fn_guard(#methods)))*
                    #(.wrap(#wrappers))*
                    .to(Self::#target)
            }
        };
        stream.extend(expanded)
    }
}

#[proc_macro_attribute]
pub fn client(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(attr as syn::AttributeArgs);
    let mut input = parse_macro_input!(item as ItemImpl);
    let mut handlers = Vec::new();
    for item in &mut input.items {
        if let syn::ImplItem::Method(ref mut item_method) = item {
            match Method::new(item_method) {
                Ok(Some(method)) => {
                    handlers.push(method);
                }
                Ok(None) => {}
                Err(err) => {
                    return err.to_compile_error().into();
                }
            }
        }
    }

    let interceptors = match ClientArgs::parse_and_strip(&mut input.attrs) {
        Ok(ClientArgs {
            interceptors
        }) => interceptors,
        Err(err) => {
            return err.to_compile_error().into();
        }
    };

    match args::Args::new(parsed) {
        Ok(args::Args {
            path,
            methods,
            wrappers,
        }) => {
            let route_idents: Vec<&syn::Ident> = handlers.iter().map(|x| &x.name).collect();
            let name = &input.self_ty;

            let expanded = quote! {
                #input
                impl #name {
                    #(#handlers)*
                }

                #[automatically_derived]
                impl actix_web::FromRequest for #name {
                    type Error = actix_web::Error;
                    type Future = futures_util::future::Ready<Result<Self, Self::Error>>;

                    #[inline]
                    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
                        match req.app_data::<actix_web::web::Data<#name>>() {
                            Some(st) => futures_util::future::ok(st.get_ref().clone()),
                            None => panic!("Failed to extract data class."),
                        }
                    }
                }

                #[automatically_derived]
                impl sept::sept_module::ServiceFactory for #name {
                    fn register(&self, app: &mut actix_web::web::ServiceConfig) {
                        app.service(
                            actix_web::web::scope(#path)
                            .app_data(actix_web::web::Data::new(self.clone()))
                            #(.guard(actix_web::guard::fn_guard(#methods)))*
                            #(.wrap(#wrappers))*
                            #(.service(Self::#route_idents(&self)))*
                        );
                    }
                }
            };
            TokenStream::from(expanded)
        }
        Err(err) => err.to_compile_error().into(),
    }
}
