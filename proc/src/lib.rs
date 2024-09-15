extern crate proc_macro;
use core::panic;
use std::iter::{self};

use proc_macro::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Comma,
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, Generics, Ident,
    Meta, Visibility,
};

#[derive(Debug)]
enum ErrType {
    Dyn((Ident, proc_macro2::Span)),
    Tree((Ident, proc_macro2::Span)),
    DynSlice((Ident, proc_macro2::Span)),
    TreeSlice((Ident, proc_macro2::Span)),
}

struct CollectedErrType {
    r#dyn: Vec<(Ident, proc_macro2::Span)>,
    tree: Vec<(Ident, proc_macro2::Span)>,
    dyniter: Vec<(Ident, proc_macro2::Span)>,
    treeiter: Vec<(Ident, proc_macro2::Span)>,
}

impl FromIterator<ErrType> for CollectedErrType {
    fn from_iter<T: IntoIterator<Item = ErrType>>(iter: T) -> Self {
        let mut r#dyn = Vec::new();
        let mut tree = Vec::new();
        let mut dyniter = Vec::new();
        let mut treeiter = Vec::new();

        iter.into_iter().for_each(|t| match t {
            ErrType::Dyn(x) => r#dyn.push(x),
            ErrType::Tree(x) => tree.push(x),
            ErrType::DynSlice(x) => dyniter.push(x),
            ErrType::TreeSlice(x) => treeiter.push(x),
        });

        Self {
            r#dyn,
            tree,
            dyniter,
            treeiter,
        }
    }
}

impl CollectedErrType {
    fn gen_sources_struct(&self, foreign: bool) -> proc_macro2::TokenStream {
        let parent = if foreign {
            quote! { self.inner }
        } else {
            quote! { self }
        };

        let conv = |x, span, y| {
            quote_spanned! {
                span=> bare_err_tree::AsErrTree::as_err_tree(& self.#x #y),
            }
        };

        let conv_vec = |x, span, y| {
            quote_spanned! {
                span=> #parent.#x.iter().map(|z|
                    bare_err_tree::AsErrTree::as_err_tree(z #y)
                ).collect::<Vec<_>>(),
            }
        };

        let gen_dyn: Vec<_> = self
            .r#dyn
            .iter()
            .map(|x| conv(&x.0, x.1, quote! {as &dyn Error}))
            .collect();
        let gen_tree: Vec<_> = self
            .tree
            .iter()
            .map(|x| conv(&x.0, x.1, quote! {}))
            .collect();
        let gen_dyniter: Vec<_> = self
            .dyniter
            .iter()
            .map(|x| conv_vec(&x.0, x.1, quote! {as &dyn Error}))
            .collect();
        let gen_treeiter: Vec<_> = self
            .treeiter
            .iter()
            .map(|x| conv_vec(&x.0, x.1, quote! {}))
            .collect();

        quote! {
            let sources = vec![
                vec![#(#gen_dyn)*],
                vec![#(#gen_tree)*],
                #(#gen_dyniter)*
                #(#gen_treeiter)*
            ].into_iter().flatten().collect();
        }
    }

    fn gen_sources_enum(&self, ident: &Ident) -> proc_macro2::TokenStream {
        if !self.dyniter.is_empty() || !self.treeiter.is_empty() {
            Error::new(
                Span::call_site().into(),
                "'*_iter' is not valid on enum fields",
            )
            .into_compile_error()
        } else {
            let conv = |x, span, y| {
                quote_spanned! {
                    span=> bare_err_tree::AsErrTree::as_err_tree(& #ident :: #x #y),
                }
            };

            let gen_dyn: Vec<_> = self
                .r#dyn
                .iter()
                .map(|x| conv(&x.0, x.1, quote! {as &dyn Error}))
                .collect();
            let gen_tree: Vec<_> = self
                .tree
                .iter()
                .map(|x| conv(&x.0, x.1, quote! {}))
                .collect();

            quote! {
                let sources = match &self.inner {
                    _ => vec![]
                }.into();
            }
        }
    }
}

fn get_struct_macros(data: &DataStruct) -> CollectedErrType {
    data.fields
        .iter()
        .flat_map(|f| {
            f.attrs.iter().filter_map(|x| {
                x.meta.require_path_only().ok().and_then(|y| {
                    y.segments
                        .iter()
                        .find_map(|seg| match seg.ident.to_string().as_str() {
                            "dyn_err" => Some(ErrType::Dyn((f.ident.clone().unwrap(), f.span()))),
                            "tree_err" => Some(ErrType::Tree((f.ident.clone().unwrap(), f.span()))),
                            "dyn_iter_err" => {
                                Some(ErrType::DynSlice((f.ident.clone().unwrap(), f.span())))
                            }
                            "tree_iter_err" => {
                                Some(ErrType::TreeSlice((f.ident.clone().unwrap(), f.span())))
                            }
                            _ => None,
                        })
                })
            })
        })
        .collect()
}

fn get_enum_macros(data: &DataEnum) -> CollectedErrType {
    data.variants
        .iter()
        .flat_map(|f| {
            f.attrs.iter().filter_map(|x| {
                x.meta.require_path_only().ok().and_then(|y| {
                    y.segments
                        .iter()
                        .find_map(|seg| match seg.ident.to_string().as_str() {
                            "dyn_err" => Some(ErrType::Dyn((f.ident.clone(), f.span()))),
                            "tree_err" => Some(ErrType::Tree((f.ident.clone(), f.span()))),
                            "dyn_iter_err" => Some(ErrType::DynSlice((f.ident.clone(), f.span()))),
                            "tree_iter_err" => {
                                Some(ErrType::TreeSlice((f.ident.clone(), f.span())))
                            }
                            _ => None,
                        })
                })
            })
        })
        .collect()
}

fn clean_struct_macros(data: &mut DataStruct) {
    data.fields.iter_mut().for_each(|f| {
        f.attrs = f
            .attrs
            .clone()
            .into_iter()
            .filter(|x| {
                x.meta
                    .require_path_only()
                    .ok()
                    .and_then(|y| {
                        y.segments
                            .iter()
                            .any(|seg| {
                                ["dyn_err", "tree_err", "dyn_iter_err", "tree_iter_err"]
                                    .contains(&seg.ident.to_string().as_str())
                            })
                            .then_some(())
                    })
                    .is_none()
            })
            .collect();
    });
}

fn clean_enum_macros(data: &mut DataEnum) {
    data.variants.iter_mut().for_each(|f| {
        f.attrs = f
            .attrs
            .clone()
            .into_iter()
            .filter(|x| {
                x.meta
                    .require_path_only()
                    .ok()
                    .and_then(|y| {
                        y.segments
                            .iter()
                            .any(|seg| {
                                ["dyn_err", "tree_err", "dyn_iter_err", "tree_iter_err"]
                                    .contains(&seg.ident.to_string().as_str())
                            })
                            .then_some(())
                    })
                    .is_none()
            })
            .collect();
    });
}

#[proc_macro_attribute]
pub fn err_tree(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args with Punctuated::<Meta, syn::Token![,]>::parse_terminated);

    let name_attribute = name_attribute(&args);

    let DeriveInput {
        attrs,
        vis,
        ident,
        generics,
        mut data,
    } = parse_macro_input!(input as DeriveInput);

    let generated = match data {
        Data::Struct(ref mut data) => {
            let errs = get_struct_macros(data);
            clean_struct_macros(data);

            if let Some(name_attribute) = name_attribute {
                let (_, ty_generics, _) = generics.split_for_impl();

                let wrapper_struct: TokenStream = quote! {
                    #vis struct #name_attribute #generics {
                        inner: #ident #ty_generics,
                    }
                }
                .into();

                let mut wrapper_struct = parse_macro_input!(wrapper_struct as DeriveInput);

                if let Data::Struct(ref mut wrapper_struct_data) = &mut wrapper_struct.data {
                    let boilerplate =
                        wrapper_boilerplate(&ident, &generics, &attrs, name_attribute);
                    let generated_impl = err_tree_struct(
                        name_attribute,
                        &wrapper_struct.generics,
                        wrapper_struct_data,
                        errs,
                        Foreign::Struct,
                    );
                    TokenStream::from_iter([
                        wrapper_struct.to_token_stream().into(),
                        boilerplate,
                        generated_impl,
                    ])
                } else {
                    panic!("The wrapper is always a struct!")
                }
            } else {
                err_tree_struct(&ident, &generics, data, errs, Foreign::NonForeign)
            }
        }
        Data::Enum(ref mut data) => {
            let errs = get_enum_macros(data);
            clean_enum_macros(data);

            if let Some(name_attribute) = name_attribute {
                let (_, ty_generics, _) = generics.split_for_impl();

                let wrapper_struct: TokenStream = quote! {
                    #vis struct #name_attribute #generics {
                        inner: #ident #ty_generics,
                    }
                }
                .into();

                let mut wrapper_struct = parse_macro_input!(wrapper_struct as DeriveInput);

                if let Data::Struct(ref mut wrapper_struct_data) = &mut wrapper_struct.data {
                    let boilerplate =
                        wrapper_boilerplate(&ident, &generics, &attrs, name_attribute);
                    let generated_impl = err_tree_struct(
                        name_attribute,
                        &wrapper_struct.generics,
                        wrapper_struct_data,
                        errs,
                        Foreign::Enum(&ident),
                    );
                    TokenStream::from_iter([
                        wrapper_struct.to_token_stream().into(),
                        boilerplate,
                        generated_impl,
                    ])
                } else {
                    panic!("The wrapper is always a struct!")
                }
            } else {
                TokenStream::from(
                    Error::new(
                        Span::call_site().into(),
                        "err_tree cannot implement directly on an enum type. Use '#[err_tree(WRAPPER)]'",
                    )
                    .into_compile_error(),
                )
            }
        }
        Data::Union(_) => TokenStream::from(
            Error::new(
                Span::call_site().into(),
                "err_tree cannot be annotated on union types",
            )
            .into_compile_error(),
        ),
    };

    TokenStream::from_iter([
        DeriveInput {
            attrs,
            vis,
            ident,
            generics,
            data,
        }
        .into_token_stream()
        .into(),
        generated,
    ])
}

// https://doc.rust-lang.org/rust-by-example/trait/derive.html
const VALID_DERIVES: [&str; 8] = [
    "Eq",
    "PartialEq",
    "Ord",
    "PartialOrd",
    "Clone",
    "Copy",
    "Hash",
    "Default",
];

fn wrapper_boilerplate(
    ident: &Ident,
    generics: &Generics,
    attrs: &[Attribute],
    name_attribute: &Ident,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let universal: TokenStream = quote! {
        #[automatically_derived]
        impl #impl_generics core::error::Error for #name_attribute #ty_generics #where_clause {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                core::error::Error::source(&self.inner)
            }
        }

        #[automatically_derived]
        impl #impl_generics core::fmt::Debug for #name_attribute #ty_generics #where_clause {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
                core::fmt::Debug::fmt(&self.inner, f)
            }
        }

        #[automatically_derived]
        impl #impl_generics core::fmt::Display for #name_attribute #ty_generics #where_clause {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
                core::fmt::Display::fmt(&self.inner, f)
            }
        }

        #[automatically_derived]
        impl #impl_generics core::convert::From<#ident #ty_generics> for #name_attribute #ty_generics #where_clause {
            #[track_caller]
            fn from(inner: #ident #ty_generics) -> Self {
                Self::_tree(inner)
            }
        }

        #[automatically_derived]
        impl #impl_generics core::convert::From<#name_attribute #ty_generics> for #ident #ty_generics #where_clause {
            fn from(value: #name_attribute #ty_generics) -> Self {
                value.inner
            }
        }

        #[automatically_derived]
        impl #impl_generics core::ops::Deref for #name_attribute #ty_generics #where_clause {
            type Target = #ident #ty_generics #where_clause;
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        #[automatically_derived]
        impl #impl_generics core::ops::DerefMut for #name_attribute #ty_generics #where_clause {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }
    }
    .into();

    let mut extra_derive = Vec::new();
    attrs.iter().for_each(|x| {
        if let Meta::List(list) = &x.meta {
            if list.path.get_ident().map(|x| x.to_string()) == Some("derive".to_string()) {
                let _ = list.parse_nested_meta(|meta| {
                    if let Some(ident) = meta.path.get_ident() {
                        extra_derive.push(ident.clone());
                    }
                    Ok(())
                });
            }
        }
    });

    let extra_derive_tokens =
        extra_derive
            .into_iter()
            .map(|extra| match extra.to_string().to_lowercase().as_str() {
                "eq" => quote! {
                    #[automatically_derived]
                    impl #impl_generics core::cmp::Eq for #name_attribute #ty_generics #where_clause {}
                }
                .into(),
                "partialeq" => quote! {
                    #[automatically_derived]
                    impl #impl_generics core::cmp::PartialEq<#name_attribute #ty_generics> for #name_attribute #ty_generics #where_clause {
                        fn eq(&self, other: &#name_attribute #ty_generics) -> bool {
                            self.inner == other.inner
                        }
                    }
                }
                .into(),
                "ord" => quote! {
                    #[automatically_derived]
                    impl #impl_generics core::cmp::Ord for #name_attribute #ty_generics #where_clause {
                        fn ord(&self, other: &Self) -> bool {
                            <#ident #ty_generics #where_clause as core::cmp::Ord>::ord(self.inner, other.inner)
                        }
                    }
                }
                .into(),
                "partialord" => quote! {
                    #[automatically_derived]
                    impl #impl_generics core::cmp::PartialOrd for #name_attribute #ty_generics #where_clause {
                        fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                            <#ident #ty_generics #where_clause as core::cmp::ParitalOrd>::partial_cmp(self.inner, other.inner)
                        }
                    }
                }
                .into(),
                "clone" => quote! {
                    #[automatically_derived]
                    impl #impl_generics core::clone::Clone for #name_attribute #ty_generics #where_clause {
                        fn clone(&self) -> Self {
                            Self {
                                inner: self.inner.clone(),
                                _err_tree_pkg: self._err_tree_pkg
                            }
                        }
                    }
                }
                .into(),
                "copy" => quote! {
                    #[automatically_derived]
                    impl #impl_generics core::marker::Copy for #name_attribute #ty_generics #where_clause {}
                }
                .into(),
                "hash" => quote! {
                    #[automatically_derived]
                    impl #impl_generics core::hash::Hash for #name_attribute #ty_generics #where_clause {
                        fn hash<H>(&self, state: &mut H)
                            where H: core::hash::Hasher
                        {
                            self.inner.hash(state)
                        }
                    }
                }
                .into(),
                "default" => quote! {
                    #[automatically_derived]
                    impl #impl_generics core::default::Default for #name_attribute #ty_generics #where_clause {
                        #[track_caller]
                        fn default() -> Self {
                            Self {
                                inner: #ident ::default(),
                                _err_tree_pkg: bare_err_tree::ErrTreePkg::default(),
                            }
                        }
                    }
                }
                .into(),
                _ => quote! {}.into(),
            });

    TokenStream::from_iter(iter::once(universal).chain(extra_derive_tokens))
}

fn name_attribute(args: &Punctuated<Meta, Comma>) -> Option<&proc_macro2::Ident> {
    args.iter().find_map(|arg| arg.path().get_ident())
}

#[derive(Debug)]
struct FieldsStrip {
    bounds: Punctuated<Field, Comma>,
    idents: Vec<Ident>,
}

fn strip_fields(fields: &Fields) -> FieldsStrip {
    let mut field_bounds = match fields.clone() {
        Fields::Named(f) => f.named,
        Fields::Unnamed(f) => f.unnamed,
        Fields::Unit => panic!("Prior checks make this impossible!"),
    };

    field_bounds.iter_mut().for_each(|f| {
        f.attrs = vec![];
        f.vis = Visibility::Inherited;
        f.colon_token = None;
    });

    let field_idents = field_bounds
        .clone()
        .into_iter()
        .flat_map(|f| f.ident)
        .collect();

    FieldsStrip {
        bounds: field_bounds,
        idents: field_idents,
    }
}

#[derive(Debug)]
enum Foreign<'a> {
    NonForeign,
    Struct,
    Enum(&'a Ident),
}

fn err_tree_struct(
    ident: &Ident,
    generics: &Generics,
    data: &mut DataStruct,
    errs: CollectedErrType,
    foreign: Foreign<'_>,
) -> TokenStream {
    let FieldsStrip {
        bounds: field_bounds,
        idents: field_names,
    } = strip_fields(&data.fields);

    let sources = match foreign {
        Foreign::NonForeign => errs.gen_sources_struct(false),
        Foreign::Struct => errs.gen_sources_struct(true),
        Foreign::Enum(ident) => errs.gen_sources_enum(ident),
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    match &mut data.fields {
        Fields::Named(fields) => {
            let field_ident = proc_macro2::Ident::new("_err_tree_pkg", Span::call_site().into());
            fields.named.push(
                Field::parse_named
                    .parse2(quote! { #field_ident: bare_err_tree::ErrTreePkg })
                    .unwrap(),
            );
            let field_ident = field_ident.into_token_stream();

            quote! {
                #[automatically_derived]
                impl #impl_generics bare_err_tree::AsErrTree for #ident #ty_generics #where_clause {
                    #[track_caller]
                    fn as_err_tree(&self) -> bare_err_tree::ErrTree<'_> {
                        #sources
                        bare_err_tree::ErrTree {
                            inner: self,
                            sources,
                            location: self.#field_ident.location,
                        }
                    }
                }

                #[automatically_derived]
                impl #impl_generics #ident #ty_generics #where_clause {
                    /// Internal constructor for a type derived with
                    /// [`err_tree`][`bare_err_tree::err_tree`].
                    ///
                    /// Call this to construct with the hidden [`ErrTreePkg`] field.
                    /// Annotate any public constructor call with `#[track_caller]`
                    /// so the error tracks the outer origin line.
                    ///
                    /// # Example:
                    /// ```ignore
                    /// #[err_tree]
                    /// struct Foo { num: i32 }
                    ///
                    /// impl Foo {
                    ///     #[track_caller]
                    ///     pub fn new(num: i32) -> Self {
                    ///         Foo::_tree(num)
                    ///     }
                    /// }
                    ///
                    /// impl Error for Foo {}
                    /// impl Display for Foo {
                    ///     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                    ///         write!(f, "")
                    ///     }
                    /// }
                    /// ```
                    #[track_caller]
                    fn _tree(#field_bounds) -> Self {
                        let #field_ident = bare_err_tree::ErrTreePkg::new();
                        Self {
                            #(#field_names,)*
                            #field_ident
                        }
                    }
                }
            }
            .into()
        }
        Fields::Unnamed(fields) => {
            let prev_len = fields.unnamed.len();
            fields.unnamed.push(
                Field::parse_unnamed
                    .parse2(quote! { bare_err_tree::ErrTreePkg })
                    .unwrap(),
            );

            quote! {
                #[automatically_derived]
                impl #impl_generics bare_err_tree::AsErrTree for #ident #ty_generics #where_clause {
                    #[track_caller]
                    fn as_err_tree(&self) -> bare_err_tree::ErrTree<'_> {
                        extern crate alloc;
                        use alloc::{boxed::Box, vec};

                        #sources
                        bare_err_tree::ErrTree {
                            inner: self,
                            sources,
                            location: self.#prev_len.location,
                        }
                    }
                }

                #[automatically_derived]
                impl #impl_generics #ident #ty_generics #where_clause {
                    #[track_caller]
                    fn _tree(#field_bounds) -> Self {
                        let err_tree_pkg = bare_err_tree::ErrTreePkg::new();
                        Self (
                            #(#field_names,)*
                            err_tree_pkg
                        )
                    }
                }
            }
            .into()
        }
        Fields::Unit => TokenStream::from(
            Error::new(
                Span::call_site().into(),
                "err_tree cannot implement directly on a unit type. Use '#[err_tree(WRAPPER)]'",
            )
            .into_compile_error(),
        ),
    }
}
