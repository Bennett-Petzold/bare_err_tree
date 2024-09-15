use core::panic;
use std::iter;

use proc_macro::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Comma,
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, Generics, Ident,
    Meta, Visibility,
};

pub fn name_attribute(args: &Punctuated<Meta, Comma>) -> Option<&proc_macro2::Ident> {
    args.iter().find_map(|arg| arg.path().get_ident())
}

#[derive(Debug)]
pub struct FieldsStrip {
    pub bounds: Punctuated<Field, Comma>,
    pub idents: Vec<Ident>,
}

pub fn strip_fields(fields: &Fields) -> FieldsStrip {
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
