extern crate proc_macro;
use core::panic;
use std::iter;

use proc_macro::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Comma,
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, Generics, Ident,
    Meta, Visibility,
};

mod errtype;
use errtype::*;
mod boiler;
use boiler::*;
mod fields;
use fields::*;

/// MACRO DOCS TODO
///
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
                err_tree_struct(&ident, &generics, data, errs, Foreign::Not)
            }
        }
        Data::Enum(ref mut data) => {
            let errs = get_enum_macros(data);
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

    let wrapper_struct: TokenStream = quote! {
        #vis struct #name_attribute #generics {
            inner: #ident #ty_generics,
        }
    }
    .into();

    let mut wrapper_struct = parse_macro_input!(wrapper_struct as DeriveInput);

    if let Data::Struct(ref mut wrapper_struct_data) = &mut wrapper_struct.data {
        let boilerplate = wrapper_boilerplate(&ident, &generics, &attrs, name_attribute);
        let generated_impl = err_tree_struct(
            name_attribute,
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
