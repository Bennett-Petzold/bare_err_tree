use core::panic;

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

pub struct CollectedErrType {
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
    pub fn gen_sources_struct(&self, foreign: bool) -> proc_macro2::TokenStream {
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

    pub fn gen_sources_enum(&self, ident: &Ident) -> proc_macro2::TokenStream {
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

pub fn get_struct_macros(data: &DataStruct) -> CollectedErrType {
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

pub fn get_enum_macros(data: &DataEnum) -> CollectedErrType {
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

pub fn clean_struct_macros(data: &mut DataStruct) {
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

pub fn clean_enum_macros(data: &mut DataEnum) {
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
