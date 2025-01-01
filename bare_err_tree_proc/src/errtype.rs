/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DataEnum, DataStruct, Expr, Field, Ident, Type};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Sizing {
    Static(Expr),
    Dynamic,
}

#[derive(Debug)]
enum ErrType {
    Dyn((Ident, proc_macro2::Span)),
    Tree((Ident, proc_macro2::Span)),
    DynSlice((Ident, proc_macro2::Span, Sizing)),
    TreeSlice((Ident, proc_macro2::Span, Sizing)),
}

pub struct CollectedErrType {
    r#dyn: Vec<(Ident, proc_macro2::Span)>,
    tree: Vec<(Ident, proc_macro2::Span)>,
    dyniter: Vec<(Ident, proc_macro2::Span, Sizing)>,
    treeiter: Vec<(Ident, proc_macro2::Span, Sizing)>,
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
                span=> &(& self.#x #y) as &dyn bare_err_tree::AsErrTree,
            }
        };

        let conv_vec = |x, span, sizing: &_, y| match sizing {
            Sizing::Dynamic => {
                quote_spanned! {
                    span=> let #x = #parent.#x.iter().map(|z|
                        z #y
                    ).collect::<alloc::vec::Vec<_>>();
                    let #x = #x.iter().map(|z|
                        z as &dyn bare_err_tree::AsErrTree
                    ).collect::<alloc::vec::Vec<_>>();
                    let #x = #x.as_slice();
                }
            }
            Sizing::Static(s) => {
                quote_spanned! {
                    span=> let #x : [_; #s] = core::array::from_fn(|i| & #parent.#x [i] #y);
                    let #x : [_; #s] = core::array::from_fn(|i| & #x [i] as &dyn bare_err_tree::AsErrTree);
                    let #x = #x.as_slice();
                }
            }
        };

        let gen_dyn: Vec<_> = self
            .r#dyn
            .iter()
            .map(|x| conv(&x.0, x.1, quote! {as &dyn core::error::Error}))
            .collect();
        let gen_tree: Vec<_> = self
            .tree
            .iter()
            .map(|x| conv(&x.0, x.1, quote! {}))
            .collect();

        let gen_dyniter: Vec<_> = self
            .dyniter
            .iter()
            .map(|x| conv_vec(&x.0, x.1, &x.2, quote! {as &dyn core::error::Error}))
            .collect();
        let gen_treeiter: Vec<_> = self
            .treeiter
            .iter()
            .map(|x| conv_vec(&x.0, x.1, &x.2, quote! {}))
            .collect();
        let iter_ids = self
            .dyniter
            .iter()
            .chain(self.treeiter.iter())
            .map(|x| &x.0);

        let any_dyn = self
            .dyniter
            .iter()
            .chain(self.treeiter.iter())
            .any(|x| x.2 == Sizing::Dynamic);
        let prelude = if any_dyn {
            quote! {
                extern crate alloc;
            }
        } else {
            quote! {}
        };

        quote! {
            #prelude
            let gen_dyn = [#(#gen_dyn)*];
            let gen_tree = [#(#gen_tree)*];
            #(#gen_dyniter)*
            #(#gen_treeiter)*
            let sources = [
                gen_dyn.as_slice(),
                gen_tree.as_slice(),
                #(#iter_ids,)*
            ];
            let sources = sources.as_slice();
            (func)(bare_err_tree::ErrTree::with_pkg(self, sources, _err_tree_pkg.clone()))
        }
    }

    pub fn gen_sources_enum(&self, ident: &Ident) -> proc_macro2::TokenStream {
        let conv = |x, span, y| {
            quote_spanned! {
                span=> #ident :: #x (x) => {
                    let x = &(x #y) as &dyn bare_err_tree::AsErrTree;
                    let x = [x];
                    let x = [x.as_slice()];
                    (func)(bare_err_tree::ErrTree::with_pkg(self, x.as_slice(), _err_tree_pkg.clone()))
                },
            }
        };

        let conv_vec = |x, span, sizing: &_, y| match sizing {
            Sizing::Dynamic => {
                quote_spanned! {
                    span=> #ident :: #x (x) => {
                        let x = x.iter().map(|z|
                            z #y
                        ).collect::<alloc::vec::Vec<_>>();
                        let x = x.iter().map(|z|
                            z as &dyn bare_err_tree::AsErrTree
                        ).collect::<alloc::vec::Vec<_>>();
                        let x = [x.as_slice()];
                        (func)(bare_err_tree::ErrTree::with_pkg(self, x.as_slice(), _err_tree_pkg.clone()))
                    }
                }
            }
            Sizing::Static(s) => {
                quote_spanned! {
                    span=> #ident :: #x (x) => {
                        let x : [_; #s] = core::array::from_fn(|i| & x [i] #y);
                        let x : [_; #s] = core::array::from_fn(|i| & x [i] as &dyn bare_err_tree::AsErrTree);
                        let x = [x.as_slice()];
                        (func)(bare_err_tree::ErrTree::with_pkg(self, x.as_slice(), _err_tree_pkg.clone()))
                    }
                }
            }
        };

        let gen_dyn: Vec<_> = self
            .r#dyn
            .iter()
            .map(|x| conv(&x.0, x.1, quote! {as &dyn core::error::Error}))
            .collect();
        let gen_tree: Vec<_> = self
            .tree
            .iter()
            .map(|x| conv(&x.0, x.1, quote! {}))
            .collect();
        let gen_dyniter: Vec<_> = self
            .dyniter
            .iter()
            .map(|x| conv_vec(&x.0, x.1, &x.2, quote! {as &dyn core::error::Error}))
            .collect();
        let gen_treeiter: Vec<_> = self
            .treeiter
            .iter()
            .map(|x| conv_vec(&x.0, x.1, &x.2, quote! {}))
            .collect();

        quote! {
            let sources = match &self.inner {
                #(#gen_dyn)*
                #(#gen_tree)*
                #(#gen_dyniter)*
                #(#gen_treeiter)*
                _ => {
                    let empty = [];
                    let empty = [empty.as_slice()];
                    (func)(bare_err_tree::ErrTree::with_pkg(self, empty.as_slice(), _err_tree_pkg.clone()))
                }
            };
        }
    }
}

fn iter_parse(
    f: &Field,
    ident: Ident,
) -> Option<Result<(Ident, proc_macro2::Span, Sizing), TokenStream>> {
    let mut ty = f.ty.clone();
    while let Type::Reference(ty_ref) = ty {
        ty = *ty_ref.elem;
    }

    if let Type::Array(ty) = ty {
        Some(Ok((ident, f.span(), Sizing::Static(ty.len.clone()))))
    } else if cfg!(feature = "alloc") {
        Some(Ok((f.ident.clone()?, f.span(), Sizing::Dynamic)))
    } else {
        Some(Err(syn::Error::new(
            f.span(),
            format!(
                "{} may be a dynamically sized collection, but the \"derive_alloc\" feature is not enabled.",
                ident
            ),
        )
        .into_compile_error()
        .into()))
    }
}

pub fn get_struct_macros(data: &DataStruct) -> Result<CollectedErrType, TokenStream> {
    data.fields
        .iter()
        .flat_map(|f| {
            f.attrs.iter().filter_map(|x| {
                x.meta.require_path_only().ok().and_then(|y| {
                    y.segments
                        .iter()
                        .find_map(|seg| match seg.ident.to_string().as_str() {
                            "dyn_err" => {
                                Some(Ok(ErrType::Dyn((f.ident.clone().unwrap(), f.span()))))
                            }
                            "tree_err" => {
                                Some(Ok(ErrType::Tree((f.ident.clone().unwrap(), f.span()))))
                            }
                            "dyn_iter_err" => iter_parse(f, f.ident.clone().unwrap())
                                .map(|z| z.map(ErrType::DynSlice)),
                            "tree_iter_err" => iter_parse(f, f.ident.clone().unwrap())
                                .map(|z| z.map(ErrType::TreeSlice)),
                            _ => None,
                        })
                })
            })
        })
        .collect()
}

pub fn get_enum_macros(data: &DataEnum) -> Result<CollectedErrType, TokenStream> {
    data.variants
        .iter()
        .flat_map(|f| {
            f.attrs.iter().filter_map(|x| {
                x.meta.require_path_only().ok().and_then(|y| {
                    y.segments
                        .iter()
                        .find_map(|seg| match seg.ident.to_string().as_str() {
                            "dyn_err" => Some(Ok(ErrType::Dyn((f.ident.clone(), f.span())))),
                            "tree_err" => Some(Ok(ErrType::Tree((f.ident.clone(), f.span())))),
                            "dyn_iter_err" => {
                                if f.fields.len() == 1 {
                                    let field =
                                        f.fields.iter().next().expect("Previously checked length");
                                    iter_parse(field, f.ident.clone())
                                        .map(|z| z.map(ErrType::DynSlice))
                                } else {
                                    Some(Ok(ErrType::DynSlice((
                                        f.ident.clone(),
                                        f.span(),
                                        Sizing::Dynamic,
                                    ))))
                                }
                            }
                            "tree_iter_err" => {
                                if f.fields.len() == 1 {
                                    let field =
                                        f.fields.iter().next().expect("Previously checked length");
                                    iter_parse(field, f.ident.clone())
                                        .map(|z| z.map(ErrType::DynSlice))
                                } else {
                                    Some(Ok(ErrType::TreeSlice((
                                        f.ident.clone(),
                                        f.span(),
                                        Sizing::Dynamic,
                                    ))))
                                }
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
