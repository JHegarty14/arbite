use lazy_static::lazy_static;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use sept_di::error::{CompileError, spanned_compile_error};
use sept_di::injectables::{get_container, get_ctor};
use sept_di::manifest;
use sept_di::manifest::{Dependency, Injectable};
use sept_di::{parsing, type_validator::TypeValidator};
use sept_di::source_data::{get_source_for_span, source_data_check};
use sept_di::type_data::TypeData;
use std::{collections::{HashMap, HashSet}, ops::Deref};
use syn::spanned::Spanned;
use syn::{FnArg, GenericArgument, Ident, Meta, Pat, PathArguments};

lazy_static! {
    static ref CLIENT_METADATA_KEYS: HashSet<String> = {
        let mut set = HashSet::<String>::new();
        set.insert("path".to_owned());
        set.insert("version".to_owned());
        set.insert("host".to_owned());
        set
    };
}

lazy_static! {
    static ref CLIENT_BINDING_KEYS: HashSet<String> = {
        let mut set = HashSet::<String>::new();
        set.insert("use_interceptors".to_owned());
        set.insert("use_guards".to_owned());
        set.insert("use_pipes".to_owned());
        set.insert("use_filters".to_owned());
        set
    };
}

pub(crate) struct ClientArgs {
    pub(crate) use_interceptors: Vec<syn::Path>,
    pub(crate) use_guards: Vec<syn::Path>,
    pub(crate) use_pipes: Vec<syn::Path>,
    pub(crate) use_filters: Vec<syn::Path>,
}

impl ClientArgs {
    pub(crate) fn parse_and_strip(attrs: &mut std::vec::Vec<syn::Attribute>) -> syn::Result<Self> {
        let mut use_interceptors = Vec::new();
        let mut use_guards = Vec::new();
        let mut use_pipes = Vec::new();
        let mut use_filters = Vec::new();
        let mut path_to_vec = HashMap::new();
        let call_site = Span::call_site();
        path_to_vec.insert(Ident::new("use_interceptors", call_site), &mut use_interceptors);
        path_to_vec.insert(Ident::new("use_guards", call_site), &mut use_guards);
        path_to_vec.insert(Ident::new("use_pipes", call_site), &mut use_pipes);
        path_to_vec.insert(Ident::new("use_filters", call_site), &mut use_filters);
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
                                    "Attribute guard expects a path!",
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
        Ok(Self {
            use_interceptors,
            use_guards,
            use_pipes,
            use_filters,
        })
    }
}

pub fn handle_client_attribute(attr: TokenStream, input: TokenStream) -> Result<TokenStream, TokenStream> {
    let span = input.span();
    let attributes = parsing::get_attribute_field_values(attr.clone())?;
    let mut type_validator = TypeValidator::new();

    for key in attributes.keys() {
        if !CLIENT_METADATA_KEYS.contains(key) {
            return spanned_compile_error(attr.span(), &format!("unknown key: {}", key));
        }
    }

    let client_path;
    let mut item_impl: syn::ItemImpl = syn::parse2(input).map_spanned_compile_error(span, "impl expected")?;
    if let syn::Type::Path(path) = item_impl.self_ty.deref() {
        client_path = path.path.to_token_stream().to_string().replace(" ", "");
    } else {
        return spanned_compile_error(attr.span(), "path expected for client");
    }

    // let name = item_impl.ident;
    let (_, ctor, fields) = get_ctor(item_impl.span(), &mut item_impl.items)?;

    let mut dependencies = Vec::<Dependency>::new();
    for arg in ctor.sig.inputs.iter_mut() {
        if let FnArg::Receiver(ref receiver) = arg {
            return spanned_compile_error(receiver.span(), &format!("self not allowed"));
        }
        if let FnArg::Typed(ref mut type_) = arg {
            if let Pat::Ident(ref ident) = *type_.pat {
                let mut dependency = Dependency::new();
                dependency.type_data = TypeData::from_syn_type(&type_.ty)?;
                let mut new_attrs = Vec::new();
                for attr in &type_.attrs {
                    match parsing::get_attribute(attr).as_str() {
                        "qualified" => {
                            type_validator.add_path(
                                &parsing::get_parenthesized_path(&attr.tokens)?,
                                attr.span(),
                            );
                            dependency.type_data.qualifier =
                                Some(Box::new(parsing::get_parenthesized_type(&attr.tokens)?))
                        }
                        _ => new_attrs.push(attr.clone()),
                    }
                }
                type_.attrs = Vec::new(); //new_attrs;
                dependency.name = ident.ident.to_string();
                dependencies.push(dependency);
            } else {
                return spanned_compile_error(type_.span(), &format!("identifier expected"));
            }
        }
    }

    let type_name;
    let mut has_lifetime = false;
    if let syn::Type::Path(ref path) = *item_impl.self_ty {
        let segments: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        type_name = segments.join("::");
        if let PathArguments::AngleBracketed(ref angle) =
            path.path.segments.last().as_ref().unwrap().arguments
        {
            for arg in &angle.args {
                if let GenericArgument::Lifetime(_) = arg {
                    has_lifetime = true;
                    break;
                }
            }
        }
    } else {
        return spanned_compile_error(item_impl.self_ty.span(), &format!("path expected"));
    }


    let client_args = match ClientArgs::parse_and_strip(&mut item_impl.attrs) {
        Ok(parsed) => parsed,
        Err(e) => return spanned_compile_error(attr.span(), &e.to_string()),
    };

    let mut injectable = Injectable::new();
    injectable.type_data = TypeData::from_local(&type_name, item_impl.self_ty.span())?;
    let scopes = parsing::get_types(attributes.get("scope"), item_impl.self_ty.span())?;
    for scope in &scopes {
        type_validator.add_dyn_type(scope, attr.span())
    }
    if let Some(scope) = attributes.get("scope") {
        for (path, span) in scope.get_paths()? {
            type_validator.add_dyn_path(&path, span);
        }
    }

    injectable.container = get_container(
        attr.span(),
        &attributes,
        &scopes,
        &mut type_validator,
        &injectable.type_data,
    )?;
    injectable.ctor_name = ctor.sig.ident.to_string();
    injectable.dependencies.extend(dependencies);
    let identifier = injectable.type_data.identifier().to_string();

    manifest::with_manifest(|mut manifest| {
        if has_lifetime {
            manifest
                .lifetimed_types
                .insert(injectable.type_data.clone());
        }
        manifest.injectables.push(injectable);
    });

    let type_check = type_validator.validate(identifier);
    get_source_for_span(item_impl.span());
    let source_data_check = source_data_check(item_impl.span());
    let result = quote! {
        #item_impl
        #type_check
        #source_data_check
    };
    // log!("{}", result.to_string());
    Ok(result)
}
