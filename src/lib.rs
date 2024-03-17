#![doc = include_str!("../README.md")]

//TODO: no_std will be a project for later
// #![no_std]
extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::{TokenTree, Group};
use quote::{ToTokens, quote, quote_spanned};
use heck::{AsUpperCamelCase, AsSnakeCase};
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse, parse_macro_input, parse_str, Attribute, Block, Error, Fields, GenericParam, Generics, Ident, ImplItem, ItemEnum, ItemImpl, FnArg, Signature, Token, Type, TypeParam, PathArguments, Variant, Visibility};
use syn::spanned::Spanned;

struct SummumType {
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    generics: Generics,
    cases: Vec<Variant>,
}

impl SummumType {
    fn parse_haskell_style(input: ParseStream, attrs: Vec<Attribute>, vis: Visibility) -> Result<Self> {
        let _ = input.parse::<Token![type]>()?;
        let name = input.parse()?;
        let generics: Generics = input.parse()?;
        let _ = input.parse::<Token![=]>()?;
        let mut cases = vec![];

        loop {
            let item_type = input.parse()?;

            let item_ident = if input.peek(Token![as]) {
                let _ = input.parse::<Token![as]>()?;
                input.parse::<Ident>()?
            } else {
                ident_from_type_full(&item_type)
            };

            let mut variant: Variant = parse(quote!{ #item_ident(#item_type) }.into())?;
            let sub_type = type_from_fields_mut(&mut variant.fields);
            cannonicalize_type_path(sub_type);

            cases.push(variant);

            if input.peek(Token![;]) {
                let _ = input.parse::<Token![;]>()?;
                break;
            }

            let _ = input.parse::<Token![|]>()?;
        }

        Ok(Self {
            attrs,
            vis,
            name,
            generics,
            cases,
        })
    }

    fn parse_enum_style(input: ParseStream, attrs: Vec<Attribute>, vis: Visibility) -> Result<Self> {
        let enum_block: ItemEnum = input.parse()?;
        let name = enum_block.ident;
        let generics = enum_block.generics;
        let cases = enum_block.variants.into_iter()
            .map(|mut variant| {
                let sub_type = type_from_fields_mut(&mut variant.fields);
                cannonicalize_type_path(sub_type);
                variant
            })
            .collect();

        Ok(Self {
            attrs,
            vis,
            name,
            generics,
            cases,
        })
    }

    fn parse(input: ParseStream, attrs: Vec<Attribute>) -> Result<Self> {
        let vis = input.parse()?;

        if input.peek(Token![type]) {
            SummumType::parse_haskell_style(input, attrs, vis)
        } else if input.peek(Token![enum]) {
            SummumType::parse_enum_style(input, attrs, vis)
        } else {
            input.step(|cursor| {
                Err(cursor.error(format!("expected `enum`, `type`, or `impl`")))
            })
        }
    }

    fn render(&self) -> TokenStream {
        let Self {
            attrs,
            vis,
            name,
            generics,
            cases,
        } = self;

        let cases_tokens = cases.iter().map(|variant| quote! {
            #variant
        }).collect::<Vec<_>>();

        let from_impls = cases.iter().map(|variant| {
            let ident = &variant.ident;
            let sub_type = type_from_fields(&variant.fields);

            quote_spanned! {variant.span() =>
                impl #generics From<#sub_type> for #name #generics {
                    fn from(val: #sub_type) -> Self {
                        #name::#ident(val)
                    }
                }
            }
        }).collect::<Vec<_>>();

        let generic_params = type_params_from_generics(&generics);
        let try_from_impls = cases.iter().map(|variant| {
            let ident = &variant.ident;
            let sub_type = type_from_fields(&variant.fields);
            if !detect_uncovered_type(&generic_params[..], &sub_type) {
                quote! {
                    impl #generics core::convert::TryFrom<#name #generics> for #sub_type {
                        type Error = ();
                        fn try_from(val: #name #generics) -> Result<Self, Self::Error> {
                            match val{#name::#ident(val)=>Ok(val), _=>Err(())}
                        }
                    }
                }
            } else {
                quote!{}
            }
        }).collect::<Vec<_>>();

        let variants_strs = cases.iter().map(|variant| {
            let ident_string = &variant.ident.to_string();
            quote_spanned! {variant.span() =>
                #ident_string
            }
        }).collect::<Vec<_>>();
        let variants_impl = quote!{
            impl #generics #name #generics {
                pub const fn variants() -> &'static[&'static str] {
                    &[#(#variants_strs),* ]
                }
            }
        };

        //TODO: I probably don't need the guide object, because I don't know how I get execute its functions within the macro, and I don't have the stamina for macro-layering insanity
        // let mut guide_name_string = name.to_string();
        // guide_name_string.push_str("Guide");
        // let guide_name = Ident::new(&guide_name_string, name.span());
        // let guide_type = quote!{
        //     struct #guide_name;

        //     impl #guide_name {
        //         pub const fn variants() -> &'static[&'static str] {
        //             &[#(#variants_strs),* ]
        //         }
        //     }
        // };

        let accessor_impls = cases.iter().map(|variant| {
            let ident = &variant.ident;
            let sub_type = type_from_fields(&variant.fields);

            let ident_string = ident.to_string();
            let is_fn_name = Ident::new(&snake_name("is", &ident_string), variant.ident.span());
            let try_as_fn_name = Ident::new(&snake_name("try_as", &ident_string), variant.ident.span());
            let as_fn_name = Ident::new(&snake_name("as", &ident_string), variant.ident.span());
            let try_as_mut_fn_name = Ident::new(&snake_name("try_as_mut", &ident_string), variant.ident.span());
            let as_mut_fn_name = Ident::new(&snake_name("as_mut", &ident_string), variant.ident.span());
            let try_into_fn_name = Ident::new(&snake_name("try_into", &ident_string), variant.ident.span());
            let into_fn_name = Ident::new(&snake_name("into", &ident_string), variant.ident.span());

            quote_spanned! {variant.span() =>
                pub fn #is_fn_name(&self) -> bool {
                    match self{Self::#ident(_)=>true, _=>false}
                }
                pub fn #try_as_fn_name(&self) -> Option<&#sub_type> {
                    match self{Self::#ident(val)=>Some(val), _=>None}
                }
                pub fn #as_fn_name(&self) -> &#sub_type {
                    self.#try_as_fn_name().unwrap()
                }
                pub fn #try_as_mut_fn_name(&mut self) -> Option<&mut #sub_type> {
                    match self{Self::#ident(val)=>Some(val), _=>None}
                }
                pub fn #as_mut_fn_name(&mut self) -> &mut #sub_type {
                    self.#try_as_mut_fn_name().unwrap()
                }
                pub fn #try_into_fn_name(self) -> Option<#sub_type> {
                    match self{Self::#ident(val)=>Some(val), _=>None}
                }
                pub fn #into_fn_name(self) -> #sub_type {
                    self.#try_into_fn_name().unwrap()
                }
            }
        }).collect::<Vec<_>>();
        let accessors_impl = quote!{
            #[allow(dead_code)]
            impl #generics #name #generics {
                #(#accessor_impls)*
            }
        };

        //TODO: re-enable this feature when https://github.com/rust-lang/rust/issues/8995 is available in stable
        // let variant_type_aliases = cases.iter().map(|variant| {
        //     let ident = &variant.ident;
        //     let variant_type_ident = Ident::new(&format!("{}T", ident.to_string()), ident.span());
        //     let sub_type = type_from_fields(&variant.fields);

        //     quote_spanned! {variant.span() =>
        //         pub type #variant_type_ident = #sub_type;
        //     }
        // }).collect::<Vec<_>>();
        // let variant_type_aliases_impl = quote!{
        //     #[allow(dead_code)]
        //     impl #generics #name #generics {
        //         #(#variant_type_aliases)*
        //     }
        // };

        quote! {
            #[allow(dead_code)]
            #(#attrs)*
            #vis enum #name #generics {
                #(#cases_tokens),*
            }

            #(#from_impls)*

            #(#try_from_impls)*

            #variants_impl

            #accessors_impl

            //TODO.  see above
            // #variant_type_aliases_impl

            // #guide_type
        }.into()
    }
}

struct SummumImpl {
    item_impl: ItemImpl,
    item_type_name: Ident,
}

impl SummumImpl {
    fn parse(input: ParseStream, attrs: Vec<Attribute>) -> Result<Self> {
        let mut item_impl: ItemImpl = input.parse()?;

        if item_impl.trait_.is_some() {
            return Err(Error::new(item_impl.span(), format!("impl for traits doesn't belong in summum block")));
        }

        item_impl.attrs = attrs;
        let item_type_name = ident_from_type_short(&*item_impl.self_ty)?;

        Ok(Self {
            item_impl,
            item_type_name
        })
    }

    fn render(&mut self, types: &HashMap<String, SummumType>) -> TokenStream {
        let item_impl = &mut self.item_impl;

        let item_type = if let Some(item_type) = types.get(&self.item_type_name.to_string()) {
            item_type
        } else {
            return quote_spanned! {
                self.item_type_name.span() => compile_error!("can't find definition for type in summum block");
            }.into();
        };

        let impl_span = item_impl.span();
        let items = core::mem::take(&mut item_impl.items);
        let mut new_items = vec![];
        for item in items.into_iter() {
            if let ImplItem::Fn(mut item) = item {

                //Create a specialized version of the function body for each variant
                let variant_blocks = item_type.cases.iter().map(|variant| {
                    let ident = &variant.ident;
                    let ident_string = ident.to_string();
                    let variant_t_name = format!("{}T", ident_string);

                    let sub_type = type_from_fields(&variant.fields);
                    let sub_type_string = quote!{ #sub_type }.to_string();

                    //Swap all the occurance of `self`, `Self`, etc. in the block
                    let block_tokenstream = replace_idents(item.block.to_token_stream(), &[
                        ("self", "_summum_self"),
                        ("Self", &sub_type_string),
                        ("VariantT", &variant_t_name),
                        ("InnerT", &sub_type_string),
                    ], &[
                        ("_inner_var", &|base| snake_name(base, &ident_string))
                    ]);
                    let block: Block = parse(quote_spanned!{item.block.span() => { #block_tokenstream } }.into()).expect("Error composing sub-block");
                    block
                }).collect::<Vec<_>>();

                //If the method name ends with "inner_var" then we'll generate a method for each variant
                let item_fn_name = item.sig.ident.to_string();
                if item_fn_name.ends_with("_inner_var") {
                    let base_fn_name = &item_fn_name[0..(item_fn_name.len() - "_inner_var".len())];
                    for (variant, block) in item_type.cases.iter().zip(variant_blocks) {
                        let mut new_item = item.clone();

                        let item_type_name = self.item_type_name.to_string();
                        let ident = &variant.ident;
                        let ident_string = ident.to_string();
                        let new_method_name = snake_name(base_fn_name, &ident_string);
                        new_item.sig.ident = Ident::new(&new_method_name, item.sig.ident.span());

                        //Swap out `VariantT` and `InnerT` in the method signature and return value
                        let variant_t_name = format!("{}T", ident_string);
                        let sub_type = type_from_fields(&variant.fields);
                        let sub_type_string = quote!{ #sub_type }.to_string();
                        let sig_tokenstream = replace_idents(new_item.sig.to_token_stream(), &[
                            ("VariantT", &variant_t_name),
                            ("InnerT", &sub_type_string),
                        ], &[]);
                        new_item.sig = parse(quote_spanned!{item.sig.span() => #sig_tokenstream }.into()).expect("Error replacing signature types");

                        //If we have a `self` input arg
                        new_item.block = if sig_contains_self_arg(&new_item.sig) {
                            parse(quote_spanned!{item.span() =>
                                {
                                    match self{
                                        Self::#ident(_summum_self) => #block ,
                                        _ => panic!("`{}::{}` method must be called with corresponding inner type", #item_type_name, #new_method_name)
                                    }
                                }
                            }.into()).unwrap()
                        } else {
                            block
                        };

                        new_items.push(ImplItem::Fn(new_item));
                    }

                } else {

                    //If the method name doesn't end with "inner_var", we'll generate just one method
                    let match_arms = item_type.cases.iter().zip(variant_blocks).map(|(variant, block)| {
                        let ident = &variant.ident;
                        quote_spanned! {item.span() =>
                            Self::#ident(_summum_self) => #block
                        }
                    }).collect::<Vec<_>>();

                    item.block = parse(quote_spanned!{item.span() =>
                        {
                            match self{
                                #(#match_arms),*
                            }
                        }
                    }.into()).unwrap();
                    new_items.push(ImplItem::Fn(item));
                }
            } else {
                new_items.push(item);
            }
        }
        item_impl.items = new_items;

        quote_spanned!{impl_span =>
            #item_impl
        }.into()
    }
}

#[derive(Default)]
struct SummumItems {
    types: HashMap<String, SummumType>,
    impls: Vec<SummumImpl>
}

impl Parse for SummumItems {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut items = Self::default();

        while !input.is_empty() {
            let attrs = input.call(Attribute::parse_outer)?;

            if input.peek(Token![impl]) {
                let next_impl = SummumImpl::parse(input, attrs)?;
                items.impls.push(next_impl);
            } else {
                let next_type = SummumType::parse(input, attrs)?;
                items.types.insert(next_type.name.to_string(), next_type);
            }
        }

        Ok(items)
    }
}

/// See the crate's top-level for usage docs
#[proc_macro]
pub fn summum(input: TokenStream) -> TokenStream {
    let mut new_stream = TokenStream::new();
    let mut items: SummumItems = parse_macro_input!(input as SummumItems);

    for item in items.types.values() {
        new_stream.extend(item.render());
    }

    for item_impl in items.impls.iter_mut() {
        new_stream.extend(item_impl.render(&items.types));
    }

    new_stream
}

/// Renders the entire type, including lifetimes and generics, into a single legal identifier token
fn ident_from_type_full(item_type: &Type) -> Ident {
    let item_ident = quote!{ #item_type }.to_string();
    let item_ident = AsUpperCamelCase(item_ident).to_string();
    Ident::new(&item_ident, item_type.span())
}

fn snake_name(base: &str, ident: &str) -> String {
    format!("{base}_{}", AsSnakeCase(ident))
}

fn type_from_fields(fields: &Fields) -> &Type {
    if let Fields::Unnamed(field) = fields {
        &field.unnamed.first().unwrap().ty
    } else {panic!()}
}

fn type_from_fields_mut(fields: &mut Fields) -> &mut Type {
    if let Fields::Unnamed(field) = fields {
        &mut field.unnamed.first_mut().unwrap().ty
    } else {panic!()}
}

/// Transforms `MyType<'a, T>` into `MyType::<'a, T>`
fn cannonicalize_type_path(item_type: &mut Type) {
    if let Type::Path(type_path) = item_type {
        if let Some(segment) = type_path.path.segments.first_mut() {
            let ident_span = segment.ident.span();
            if let PathArguments::AngleBracketed(args) = &mut segment.arguments {
                if args.colon2_token.is_none() {
                    args.colon2_token = Some(Token![::](ident_span));
                }
            }
        }
    }
}

/// Detect the situation where we'd get the error: https://doc.rust-lang.org/error_codes/E0210.html
/// `type parameter `T` must be covered by another type when it appears before the first local type...`
fn detect_uncovered_type(generic_type_params: &[&TypeParam], item_type: &Type) -> bool {
    match item_type {
        Type::Path(type_path) => {
            if let Some(type_ident) = type_path.path.get_ident() {
                for generic_type_params in generic_type_params {
                    if generic_type_params.ident.to_string() == type_ident.to_string() {
                        return true;
                    }
                }
            }
            false
        }
        Type::Reference(type_ref) => detect_uncovered_type(generic_type_params, &type_ref.elem),
        _ => false
    }
}

fn type_params_from_generics(generics: &Generics) -> Vec<&TypeParam> {
    let mut results = vec![];
    for generic_param in generics.params.iter() {
        if let GenericParam::Type(type_param) = generic_param {
            results.push(type_param);
        }
    }
    results
}

/// Helper object to parse an identifier from a compound type with generics
struct TypeIdentParseHelper(Ident);

impl Parse for TypeIdentParseHelper {
    fn parse(input: ParseStream) -> Result<Self> {

        let mut result = Err(Error::new(input.span(), format!("invalid type")));
        while !input.is_empty() {
            if input.peek(Ident) {
                let ident = input.parse::<Ident>()?;
                if result.is_err() {
                    result = Ok(Self(ident));
                }
            } else {
                _ = input.parse::<TokenTree>()?;
            }
        }

        result
    }
}

/// Renders the name of a type into a single legal identifier token, stripping away generics and lifetimes
fn ident_from_type_short(item_type: &Type) -> Result<Ident> {
    let type_stream = quote!{ #item_type }.into();
    let ident: TypeIdentParseHelper = parse(type_stream)?;
    Ok(ident.0)
}

/// Do a depth-first traversal of a TokenStream replacing each ident in a map with another ident
fn replace_idents(input: proc_macro2::TokenStream, map: &[(&str, &str)], ends_with_map: &[(&str, &dyn Fn(&str) -> String)]) -> proc_macro2::TokenStream {
    let mut new_stream = proc_macro2::TokenStream::new();

    for item in input.into_iter() {
        match item {
            TokenTree::Ident(ident) => {
                let ident_string = ident.to_string();
                if let Some(replacement_string) = map.iter().find_map(|(key, val)| {
                    if key == &ident_string {
                        Some(val.to_string())
                    } else {
                        None
                    }
                }).or_else(|| {
                    ends_with_map.iter().find_map(|(ends_with_key, func)| {
                        if ident_string.ends_with(ends_with_key) {
                            Some(func(&ident_string[0..(ident_string.len() - ends_with_key.len())]))
                        } else {
                            None
                        }
                    })
                }) {
                    let replacement_stream = parse_str::<proc_macro2::TokenStream>(&replacement_string).expect("Error rendering type back to tokens");
                    let replacement_stream: proc_macro2::TokenStream = replacement_stream.into_iter()
                        .map(|mut item| {item.set_span(ident.span()); item} ).collect();
                    new_stream.extend([replacement_stream]);
                } else {
                    new_stream.extend([TokenTree::Ident(ident)]);
                }
            },
            TokenTree::Group(group) => {
                let new_group_stream = replace_idents(group.stream(), map, ends_with_map);
                let mut new_group = Group::new(group.delimiter(), new_group_stream);
                new_group.set_span(group.span());
                new_stream.extend([TokenTree::Group(new_group)]);
            },
            _ => {new_stream.extend([item]);}
        }
    }

    new_stream
}

fn sig_contains_self_arg(sig: &Signature) -> bool {
    if let Some(first_arg) = sig.inputs.first() {
        if let FnArg::Receiver(_rcvr) = first_arg {
            return true;
        }
    }
    false
}

/// An example of a generated sum type
#[cfg(feature = "generated_example")]
#[allow(missing_docs)]
pub mod generated_example {
    use crate::summum;
    summum! {
        /// An example type generated with the invocation:
        ///
        /// ```
        /// DocCEPTION!!
        /// ```
        #[derive(Debug, Copy, Clone, PartialEq)]
        enum GeneratedExample<'a, T> {
            Slice(&'a [T]),
            Vec(Vec<T>),
        }

        impl<'a, T> SliceOrPie<'a, T> {
            /// Returns a reference to the `T` as `idx`
            fn get(&self, idx: usize) -> Option<&T> {
                self.get(idx)
            }

            //TODO: I want to show examples of all the features... but alas I'd need a separate
            // crate to actually publish them on docs.rs.  And I want to keep to a single crate.
        }
    }
}

//TODO, Make sure overlapping types shared by different variants are handled nicely.
//Maybe add an attribute so From<> and TryFrom<> impl can be disabled to avoid conflict when two variants have the same type,
//or at the very least make a nice error
