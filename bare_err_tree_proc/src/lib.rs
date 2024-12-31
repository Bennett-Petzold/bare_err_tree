/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Derive macros for `bare_err_tree`.

extern crate proc_macro;
use core::panic;

use proc_macro::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, token::Brace, Attribute, Data,
    DataStruct, DeriveInput, Error, Field, Fields, FieldsNamed, Generics, Ident, Meta, Visibility,
};

mod errtype;
use errtype::*;
mod boiler;
use boiler::*;
mod fields;
use fields::*;

/// Implements a type as an error tree.
///
/// The struct must define [`Error`](`core::error::Error`) and be annotated with `#[err_tree]` above
/// any attributes relying on a full field definition. The type must then be
/// internally constructed with `Self::_tree` to capture extra error
/// information in a hidden field.
///
/// Any derive such as [`Clone`] that relies on all fields being present must
/// occur after the `#[err_tree]` macro. The `_err_tree_pkg` field will
/// otherwise be added late and break the derivation.
///
/// # `Self::_tree`
/// This is an internal-use constructor that takes all struct fields in order.
/// Use `#[track_caller]` on any functions calling `Self::_tree` to store the
/// callsite correctly.
/// [Open an issue or PR](<https://github.com/Bennett-Petzold/bare_err_tree>)
/// if this hidden field degrades a struct's API (aside from requiring a
/// constructor method).
///
/// #### Example
/// ```
/// # use std::{error::Error, fmt::{self, Debug, Display, Formatter}};
/// use bare_err_tree::{err_tree, tree_unwrap};
///
/// #[err_tree]
/// #[derive(Debug)]
/// struct Foo {
///     num: i32,
/// }
///
/// impl Foo {
///     #[track_caller]
///     pub fn new(num: i32) -> Self {
///         Foo::_tree(num)
///     }
/// }
///
/// impl Error for Foo {
///     fn source(&self) -> Option<&(dyn Error + 'static)> {
///         # /*
///         ...
///         # */
///         # unimplemented!()
///     }
/// }
/// impl Display for Foo {
///     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
///         # /*
///         ...
///         # */
///         # unimplemented!()
///     }
/// }
/// ```
///
/// # Field Annotations
/// The macro needs annotations for underlying source fields.
///
/// #### Single Item
/// * `tree_err`: Mark a field as a `ErrTree` implementing [`Error`](`core::error::Error`).
/// * `dyn_err`: Mark a field as a generic [`Error`](`core::error::Error`).
///
/// #### Collection
/// `*_iter_err` works on any type with a `.iter()` method returning its items.
///
/// * `tree_iter_err`: Mark a field as a collection of `ErrTree` implementing [`Error`](`core::error::Error`)s.
/// * `dyn_iter_err`: Mark a field as a collection of generic [`Error`](`core::error::Error`)s.
///
/// `*_iter_err` does not allocate for arrays with a known length.
/// The `derive_alloc` feature enables generation of allocating code to support
/// dynamically sized collections.
///
/// #### Example
/// ```
/// # use std::{any::Any, error::Error, fmt::{self, Debug, Display, Formatter}};
/// use bare_err_tree::{err_tree, tree_unwrap, AsErrTree, ErrTree};
///
/// #[err_tree]
/// #[derive(Debug)]
/// struct Foo {
///     #[dyn_err]
///     io_err: std::io::Error,
///     #[dyn_iter_err]
///     extra_io_errs: [std::io::Error; 5],
/// }
///
/// impl Foo {
///     #[track_caller]
///     pub fn new(io_err: std::io::Error, extra_io_errs: [std::io::Error; 5]) -> Self {
///         Foo::_tree(io_err, extra_io_errs)
///     }
/// }
///
/// impl Error for Foo {
///     fn source(&self) -> Option<&(dyn Error + 'static)> {
///         # /*
///         ...
///         # */
///         # unimplemented!()
///     }
/// }
/// impl Display for Foo {
///     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
///         # /*
///         ...
///         # */
///         # unimplemented!()
///     }
/// }
///
/// fn main() {
///     // Make a Foo of all EOF errors
///     let eof_gen = || std::io::Error::from(std::io::ErrorKind::UnexpectedEof);
///     let err = Foo::new(eof_gen(), std::array::from_fn(|_| eof_gen()));
///
///     // Confirm exactly six sources from annotation
///     err.as_err_tree(&mut |tree| {
///         let sources = tree.sources();
///         assert_eq!(sources.iter().map(|s| s.iter()).flatten().count(), 6);
///     });
/// }
/// ```
///
/// # Generating a Wrapper
/// `#[err_tree(WRAPPER)]` will generate a wrapper struct for storing metadata.
/// Enums need this form, as a hidden field cannot be added to the enum.
/// `WRAPPER` provides [`From`](`core::convert::From`) both ways and
/// [`Deref`](`core::ops::Deref`)/[`DerefMut`](`core::ops::DerefMut`) to be
/// maximally transparent.
/// Some derives are automatically re-derived for the wrapper; any other traits
/// that need to be implemented for the wrapper can be written manually.
///
/// #### Wrapper automatic re-derives
// https://doc.rust-lang.org/rust-by-example/trait/derive.html
/// [`Eq`](`core::cmp::Eq`), [`PartialEq`](`core::cmp::PartialEq`),
/// [`Ord`](`core::cmp::Ord`), [`PartialOrd`](`core::cmp::PartialOrd`),
/// [`Clone`](`core::clone::Clone`), [`Hash`](`core::hash::Hash`),
/// [`Default`](`core::default::Default).
///
/// #### Enum Example
/// ```
/// # use std::{error::Error, fmt::{self, Debug, Display, Formatter}};
/// use bare_err_tree::{err_tree, tree_unwrap};
///
/// // Generates `FooWrap<T: Debug>`
/// #[err_tree(FooWrap)]
/// #[derive(Debug)]
/// enum Foo<T: Debug> {
///     Val(T),
///     #[dyn_err]
///     Single(std::io::Error),
///     #[dyn_iter_err]
///     Many([std::io::Error; 5]),
/// }
///
/// impl<T: Debug> Error for Foo<T> {
///     fn source(&self) -> Option<&(dyn Error + 'static)> {
///         # /*
///         ...
///         # */
///         # unimplemented!()
///     }
/// }
/// impl<T: Debug> Display for Foo<T> {
///     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
///         # /*
///         ...
///         # */
///         # unimplemented!()
///     }
/// }
///
/// fn main() {
///     let wrapped = FooWrap::from(Foo::Val(8_i32));
///     assert!(matches!(*wrapped, Foo::Val(8_i32)));
/// }
///
/// ```
///
/// # Full Usage Example:
/// ```
/// # use std::{error::Error, fmt::{self, Debug, Display, Formatter}};
/// use bare_err_tree::{err_tree, tree_unwrap};
///
/// #[err_tree]
/// #[derive(Debug)]
/// struct Foo {
///     #[dyn_err]
///     io_err: std::io::Error,
///     #[dyn_iter_err]
///     extra_io_errs: [std::io::Error; 5],
/// }
///
/// impl Foo {
///     #[track_caller]
///     pub fn new(io_err: std::io::Error, extra_io_errs: [std::io::Error; 5]) -> Self {
///         Foo::_tree(io_err, extra_io_errs)
///     }
/// }
///
/// impl Error for Foo {
///     fn source(&self) -> Option<&(dyn Error + 'static)> {
///         Some(&self.io_err)
///     }
/// }
/// impl Display for Foo {
///     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
///         # /*
///         ...
///         # */
///         # Display::fmt(&self.io_err, f)
///     }
/// }
///
/// /// Always return the error with tree formatting support
/// pub fn always_fail() -> Result<(), Foo> {
///     # let get_err = || std::io::Error::from(std::io::ErrorKind::UnexpectedEof);
///     Err(Foo::new(
///     # /*
///         ...
///     # */
///     # get_err(), std::array::from_fn(|_| get_err()),
///     ))
/// }
///
/// const MAX_DEPTH: usize = 10;
/// const MAX_CHARS: usize = MAX_DEPTH * 6;
///
/// pub fn main() {
///     # let _ = std::panic::catch_unwind(|| {
///     let result = always_fail();
///
///     /// Fancy display panic with a maximum tree depth of 10 errors
///     tree_unwrap::<MAX_CHARS, _, _, _>(result);
///     # });
/// }
/// ```
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
        Data::Struct(ref mut data) => match get_struct_macros(data) {
            Ok(errs) => {
                clean_struct_macros(data);

                if let Some(name_attribute) = name_attribute {
                    foreign_err_tree(
                        &ident,
                        &vis,
                        &attrs,
                        name_attribute,
                        &generics,
                        errs,
                        Foreign::Struct,
                    )
                } else {
                    err_tree_struct(&ident, &vis, &generics, data, errs, Foreign::Not)
                }
            }
            Err(e) => return e,
        },
        Data::Enum(ref mut data) => match get_enum_macros(data) {
            Ok(errs) => {
                clean_enum_macros(data);

                if let Some(name_attribute) = name_attribute {
                    foreign_err_tree(
                        &ident,
                        &vis,
                        &attrs,
                        name_attribute,
                        &generics,
                        errs,
                        Foreign::Enum(&ident),
                    )
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
            Err(e) => return e,
        },
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

#[derive(Debug)]
enum Foreign<'a> {
    Not,
    Struct,
    Enum(&'a Ident),
}

fn foreign_err_tree(
    ident: &Ident,
    vis: &Visibility,
    attrs: &[Attribute],
    name_attribute: &Ident,
    generics: &Generics,
    errs: CollectedErrType,
    foreign_type: Foreign,
) -> TokenStream {
    let (_, ty_generics, _) = generics.split_for_impl();

    let doc_attrs: Vec<_> = attrs
        .iter()
        .filter(|x| {
            if let Ok(x) = x.meta.require_name_value() {
                if let Some(x) = x.path.get_ident() {
                    x == "doc"
                } else {
                    false
                }
            } else {
                false
            }
        })
        .collect();

    let ident_link = format!("Wrapper for [`{ident}`] generated by [`bare_err_tree`].");
    let wrapper_struct: TokenStream = quote! {
        #[doc = #ident_link]
        ///
        #(#doc_attrs)*
        #vis struct #name_attribute #generics {
            inner: #ident #ty_generics,
        }
    }
    .into();

    let mut wrapper_struct = parse_macro_input!(wrapper_struct as DeriveInput);

    if let Data::Struct(ref mut wrapper_struct_data) = &mut wrapper_struct.data {
        let boilerplate = wrapper_boilerplate(ident, generics, attrs, name_attribute);
        let generated_impl = err_tree_struct(
            name_attribute,
            vis,
            &wrapper_struct.generics,
            wrapper_struct_data,
            errs,
            foreign_type,
        );
        TokenStream::from_iter([
            wrapper_struct.to_token_stream().into(),
            boilerplate,
            generated_impl,
        ])
    } else {
        panic!("The wrapper is always a struct!")
    }
}

fn err_tree_struct(
    ident: &Ident,
    vis: &Visibility,
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
        Foreign::Not => errs.gen_sources_struct(false),
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
                    fn as_err_tree(&self, func: &mut dyn FnMut(bare_err_tree::ErrTree<'_>)) {
                        let _err_tree_pkg = self.#field_ident .clone();
                        #sources
                    }
                }

                #[automatically_derived]
                impl #impl_generics #ident #ty_generics #where_clause {
                    #[track_caller]
                    #[allow(clippy::too_many_arguments)]
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
            let prev_len = syn::Index::from(fields.unnamed.len());
            fields.unnamed.push(
                Field::parse_unnamed
                    .parse2(quote! { bare_err_tree::ErrTreePkg })
                    .unwrap(),
            );

            quote! {
                #[automatically_derived]
                impl #impl_generics bare_err_tree::AsErrTree for #ident #ty_generics #where_clause {
                    #[track_caller]
                    fn as_err_tree(&self, func: &mut dyn FnMut(bare_err_tree::ErrTree<'_>)) {
                        let _err_tree_pkg = self.#prev_len .clone();
                        #sources
                    }
                }

                #[automatically_derived]
                impl #impl_generics #ident #ty_generics #where_clause {
                    #[track_caller]
                    #[allow(clippy::too_many_arguments)]
                    fn _tree(#field_bounds) -> Self {
                        let _err_tree_pkg = bare_err_tree::ErrTreePkg::new();
                        Self (
                            #(#field_names,)*
                            _err_tree_pkg
                        )
                    }
                }
            }
            .into()
        }
        Fields::Unit => {
            let field_ident = proc_macro2::Ident::new("_err_tree_pkg", Span::call_site().into());
            let mut named = Punctuated::default();
            named.push(
                Field::parse_named
                    .parse2(quote! { #field_ident: bare_err_tree::ErrTreePkg })
                    .unwrap(),
            );
            let field_ident = field_ident.into_token_stream();
            data.fields = Fields::Named(FieldsNamed {
                brace_token: Brace::default(),
                named,
            });

            quote! {
                #[automatically_derived]
                impl #impl_generics bare_err_tree::AsErrTree for #ident #ty_generics #where_clause {
                    #[track_caller]
                    fn as_err_tree(&self, func: &mut dyn FnMut(bare_err_tree::ErrTree<'_>)) {
                        let _err_tree_pkg = self.#field_ident .clone();
                        #sources
                    }
                }

                #[automatically_derived]
                impl #impl_generics #ident #ty_generics #where_clause {
                    #[track_caller]
                    #[allow(clippy::too_many_arguments)]
                    fn _tree() -> Self {
                        let #field_ident = bare_err_tree::ErrTreePkg::new();
                        Self {
                            #field_ident
                        }
                    }
                }

                #[automatically_derived]
                impl #impl_generics core::default::Default for #ident #ty_generics #where_clause {
                    #[track_caller]
                    fn default() -> Self {
                        Self::_tree()
                    }
                }

                #[automatically_derived]
                impl #impl_generics #ident #ty_generics #where_clause {
                    #[track_caller]
                    #vis fn new() -> Self {
                        Self::_tree()
                    }
                }
            }
            .into()
        }
    }
}
