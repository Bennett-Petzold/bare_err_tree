/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DataEnum, DataStruct, Expr, Field, Ident, Type};

#[derive(Debug)]
enum ErrType {
    /// &dyn ErrTree, not in a collection
    Dyn((Ident, proc_macro2::Span)),
    /// Known ErrTree, not in a collection
    Tree((Ident, proc_macro2::Span)),
    /// &dyn ErrTree, in a collection
    DynSlice((Ident, proc_macro2::Span)),
    /// Known ErrTree, in a collection
    TreeSlice((Ident, proc_macro2::Span)),
}

/// Sorted collection of [`ErrType`]s
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
    /// Generate the `with_pkg` call on all notated sources in a struct.
    ///
    /// Avoids any allocation on iterators of static arrays.
    pub fn gen_sources_struct(&self, foreign: bool) -> proc_macro2::TokenStream {
        // Trivial name change covers both foreign and direct impl
        let parent = if foreign {
            quote! { self.inner }
        } else {
            quote! { self }
        };

        let conv = |x, span, y| {
            quote_spanned! {
                span=> let #x = bare_err_tree::ErrTreeConv::from(& self.#x #y);
                    let #x = core::iter::once(#x);
            }
        };

        let conv_dyn_vec = |x, span| {
            quote_spanned! {
                span=> let #x = #parent.#x.iter()
                    .map(|x| bare_err_tree::ErrTreeConv::from(x as &dyn core::error::Error));
            }
        };

        let conv_vec = |x, span| {
            quote_spanned! {
                span=> let #x = #parent.#x.iter().map(|x| bare_err_tree::ErrTreeConv::from(x));
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

        let noniter_ids = self.r#dyn.iter().chain(self.tree.iter()).map(|x| &x.0);

        let gen_dyniter: Vec<_> = self
            .dyniter
            .iter()
            .map(|x| conv_dyn_vec(&x.0, x.1))
            .collect();
        let gen_treeiter: Vec<_> = self.treeiter.iter().map(|x| conv_vec(&x.0, x.1)).collect();
        let iter_ids = self
            .dyniter
            .iter()
            .chain(self.treeiter.iter())
            .map(|x| &x.0);

        quote! {
            #(#gen_dyn)*
            #(#gen_tree)*
            let mut singles = core::iter::empty()#(.chain(#noniter_ids))*;

            #(#gen_dyniter)*
            #(#gen_treeiter)*
            let mut nested = core::iter::empty()#(.chain(#iter_ids))*;

            let sources = &mut singles.chain(nested);
            (func)(bare_err_tree::ErrTree::with_pkg(self, sources, _err_tree_pkg))
        }
    }

    /// Generate the `with_pkg` call on all notated sources in a enum.
    ///
    /// Avoids any allocation on iterators of static arrays.
    pub fn gen_sources_enum(&self, ident: &Ident) -> proc_macro2::TokenStream {
        let conv = |x, span, y| {
            quote_spanned! {
                span=> #ident :: #x (x) => {
                    let x = bare_err_tree::ErrTreeConv::from(x #y);
                    let x = &mut core::iter::once(x);
                    (func)(bare_err_tree::ErrTree::with_pkg(self, x, _err_tree_pkg))
                },
            }
        };

        let conv_vec = |x, span, y| {
            quote_spanned! {
                span=> #ident :: #x (x) => {
                    let x = &mut x.iter().map(|z| bare_err_tree::ErrTreeConv::from(z #y));
                    (func)(bare_err_tree::ErrTree::with_pkg(self, x, _err_tree_pkg))
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
            .map(|x| conv_vec(&x.0, x.1, quote! {as &dyn core::error::Error}))
            .collect();
        let gen_treeiter: Vec<_> = self
            .treeiter
            .iter()
            .map(|x| conv_vec(&x.0, x.1, quote! {}))
            .collect();

        quote! {
            let sources = match &self.inner {
                #(#gen_dyn)*
                #(#gen_tree)*
                #(#gen_dyniter)*
                #(#gen_treeiter)*
                _ => {
                    (func)(bare_err_tree::ErrTree::with_pkg(self, &mut core::iter::empty(), _err_tree_pkg))
                }
            };
        }
    }
}

/// Parse iterator types.
///
/// Distinguishes between sized and unsized arrays to generate the
/// correct identity name and sizing types.
fn iter_parse(f: &Field, ident: Ident) -> Option<Result<(Ident, proc_macro2::Span), TokenStream>> {
    let mut ty = f.ty.clone();
    while let Type::Reference(ty_ref) = ty {
        ty = *ty_ref.elem;
    }

    Some(Ok((ident, f.span())))
}

/// Finds all child error annotations on a struct.
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

/// Finds all child error annotations on an enum.
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
                                    Some(Ok(ErrType::DynSlice((f.ident.clone(), f.span()))))
                                }
                            }
                            "tree_iter_err" => {
                                if f.fields.len() == 1 {
                                    let field =
                                        f.fields.iter().next().expect("Previously checked length");
                                    iter_parse(field, f.ident.clone())
                                        .map(|z| z.map(ErrType::DynSlice))
                                } else {
                                    Some(Ok(ErrType::TreeSlice((f.ident.clone(), f.span()))))
                                }
                            }
                            _ => None,
                        })
                })
            })
        })
        .collect()
}

/// Remove this library's annotation, as they aren't actually valid macros.
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

/// Remove this library's annotation, as they aren't actually valid macros.
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
