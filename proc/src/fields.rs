/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::panic;

use syn::{punctuated::Punctuated, token::Comma, Field, Fields, Ident, Meta, Visibility};

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
