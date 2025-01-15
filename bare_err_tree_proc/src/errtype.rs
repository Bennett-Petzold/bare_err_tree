/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, DataEnum, DataStruct, Field, Ident, Type};

#[derive(Debug)]
pub enum ErrType {
    /// &dyn ErrTree, not in a collection
    Dyn,
    /// Known ErrTree, not in a collection
    Tree,
    /// &dyn ErrTree, in a collection
    DynIter,
    /// Known ErrTree, in a collection
    TreeIter,
}

#[derive(Debug)]
pub struct TreeErr {
    ident: Ident,
    span: proc_macro2::Span,
    var: ErrType,
}

impl TreeErr {
    pub fn new(ident: Ident, span: proc_macro2::Span, var: ErrType) -> Self {
        Self { ident, span, var }
    }
}

/// Generate the `with_pkg` call on all notated sources in a struct.
pub fn gen_sources_struct(errs: &[TreeErr], foreign: bool) -> proc_macro2::TokenStream {
    // Trivial name change covers both foreign and direct impl
    let parent = if foreign {
        quote! { self.inner }
    } else {
        quote! { self }
    };

    let conv = |x, span| {
        quote_spanned! {
            span=> let #x = & self.#x as &dyn ::bare_err_tree::AsErrTree;
                let #x = core::iter::once(#x);
        }
    };

    let conv_dyn = |x, span| {
        quote_spanned! {
            span=> let #x = ::bare_err_tree::WrapErr::tree(& self.#x);
                let #x = core::iter::once(#x);
        }
    };

    let conv_dyn_iter = |x, span| {
        quote_spanned! {
            span=> let #x = #parent.#x.iter()
                .map(::bare_err_tree::WrapErr::tree);
        }
    };

    let conv_iter = |x, span| {
        quote_spanned! {
            span=> let #x = #parent.#x.iter().map(|x| x as &dyn ::bare_err_tree::AsErrTree);
        }
    };

    let gen_vars = errs.iter().map(|err| match err.var {
        ErrType::Dyn => conv_dyn(&err.ident, err.span),
        ErrType::Tree => conv(&err.ident, err.span),
        ErrType::DynIter => conv_dyn_iter(&err.ident, err.span),
        ErrType::TreeIter => conv_iter(&err.ident, err.span),
    });
    let ids = errs.iter().map(|err| &err.ident);

    quote! {
        #(#gen_vars)*
        let mut sources = &mut core::iter::empty()#(.chain(#ids))*;

        (func)(::bare_err_tree::ErrTree::with_pkg(self, sources, _err_tree_pkg))
    }
}

/// Generate the `with_pkg` call on all notated sources in a enum.
pub fn gen_sources_enum(errs: &[TreeErr], ident: &Ident) -> proc_macro2::TokenStream {
    let conv = |x, span| {
        quote_spanned! {
            span=> #ident :: #x (x) => {
                let x = x as &dyn ::bare_err_tree::AsErrTree;
                let x = &mut core::iter::once(x);
                (func)(::bare_err_tree::ErrTree::with_pkg(self, x, _err_tree_pkg))
            },
        }
    };

    let conv_dyn = |x, span| {
        quote_spanned! {
            span=> #ident :: #x (x) => {
                let x = ::bare_err_tree::WrapErr::tree(x);
                let x = &mut core::iter::once(x);
                (func)(::bare_err_tree::ErrTree::with_pkg(self, x, _err_tree_pkg))
            },
        }
    };

    let conv_iter = |x, span| {
        quote_spanned! {
            span=> #ident :: #x (x) => {
                let x = &mut x.iter().map(|z| z as &dyn AsErrTree);
                (func)(::bare_err_tree::ErrTree::with_pkg(self, x, _err_tree_pkg))
            }
        }
    };

    let conv_iter_dyn = |x, span| {
        quote_spanned! {
            span=> #ident :: #x (x) => {
                let x = &mut x.iter().map(::bare_err_tree::WrapErr::tree);
                (func)(::bare_err_tree::ErrTree::with_pkg(self, x, _err_tree_pkg))
            }
        }
    };

    let gen_arms = errs.iter().map(|err| match err.var {
        ErrType::Dyn => conv_dyn(&err.ident, err.span),
        ErrType::Tree => conv(&err.ident, err.span),
        ErrType::DynIter => conv_iter_dyn(&err.ident, err.span),
        ErrType::TreeIter => conv_iter(&err.ident, err.span),
    });

    quote! {
        let sources = match &self.inner {
            #(#gen_arms)*
            _ => {
                (func)(::bare_err_tree::ErrTree::with_pkg(self, &mut core::iter::empty(), _err_tree_pkg))
            }
        };
    }
}

/// Parse iterator types.
///
/// Distinguishes between sized and unsized arrays to generate the
/// correct identity name and sizing types.
fn iter_parse(f: &Field, ident: Ident, var: ErrType) -> TreeErr {
    let mut ty = f.ty.clone();
    while let Type::Reference(ty_ref) = ty {
        ty = *ty_ref.elem;
    }

    TreeErr::new(ident, f.span(), var)
}

/// Finds all child error annotations on a struct.
pub fn get_struct_macros(data: &DataStruct) -> impl Iterator<Item = TreeErr> + use<'_> {
    data.fields.iter().flat_map(|f| {
        f.attrs.iter().filter_map(|x| {
            x.meta.require_path_only().ok().and_then(|y| {
                y.segments
                    .iter()
                    .find_map(|seg| match seg.ident.to_string().as_str() {
                        "dyn_err" => Some(TreeErr::new(
                            f.ident.clone().unwrap(),
                            f.span(),
                            ErrType::Dyn,
                        )),
                        "tree_err" => Some(TreeErr::new(
                            f.ident.clone().unwrap(),
                            f.span(),
                            ErrType::Tree,
                        )),
                        "dyn_iter_err" => {
                            Some(iter_parse(f, f.ident.clone().unwrap(), ErrType::DynIter))
                        }
                        "tree_iter_err" => {
                            Some(iter_parse(f, f.ident.clone().unwrap(), ErrType::TreeIter))
                        }
                        _ => None,
                    })
            })
        })
    })
}

/// Finds all child error annotations on an enum.
pub fn get_enum_macros(data: &DataEnum) -> impl Iterator<Item = TreeErr> + use<'_> {
    data.variants.iter().flat_map(|f| {
        f.attrs.iter().filter_map(|x| {
            x.meta.require_path_only().ok().and_then(|y| {
                y.segments
                    .iter()
                    .find_map(|seg| match seg.ident.to_string().as_str() {
                        "dyn_err" => Some(TreeErr::new(f.ident.clone(), f.span(), ErrType::Dyn)),
                        "tree_err" => Some(TreeErr::new(f.ident.clone(), f.span(), ErrType::Tree)),
                        "dyn_iter_err" => {
                            if f.fields.len() == 1 {
                                let field =
                                    f.fields.iter().next().expect("Previously checked length");
                                Some(iter_parse(field, f.ident.clone(), ErrType::DynIter))
                            } else {
                                Some(TreeErr::new(f.ident.clone(), f.span(), ErrType::DynIter))
                            }
                        }
                        "tree_iter_err" => {
                            if f.fields.len() == 1 {
                                let field =
                                    f.fields.iter().next().expect("Previously checked length");
                                Some(iter_parse(field, f.ident.clone(), ErrType::TreeIter))
                            } else {
                                Some(TreeErr::new(f.ident.clone(), f.span(), ErrType::TreeIter))
                            }
                        }
                        _ => None,
                    })
            })
        })
    })
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
