/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::iter;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Generics, Ident, Meta};

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

pub fn wrapper_boilerplate(
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
