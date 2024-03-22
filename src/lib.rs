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
use syn::{parse, parse2, parse::ParseBuffer, parse_quote, parse_macro_input, parse_str, Attribute, Block, Error, Fields, Field, GenericParam, Generics, Ident, ImplItem, ItemEnum, ItemImpl, FnArg, punctuated::Punctuated, Signature, Token, Type, TypeParam, PathArguments, Variant, Visibility};
use syn::spanned::Spanned;

struct SummumType {
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    generics: Generics,
    cases: Vec<Variant>,
    sub_types: Vec<SubType>,
    struct_fields: Vec<Field>,
}

mod keywords {
    syn::custom_keyword!(variants);
}

struct SubType {
    variant_name: Ident,
    bindings: Vec<(Ident, Type)>
}

impl SubType {
    fn struct_type_ident(&self, base_name: &Ident) -> Ident {
        let sub_type_name_string = format!("{}{}", base_name, self.variant_name);
        Ident::new(&sub_type_name_string, self.variant_name.span())
    }
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
            sub_types: vec![],
            struct_fields: vec![],
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
            sub_types: vec![],
            struct_fields: vec![],
        })
    }

    fn parse_struct(input: ParseStream, attrs: Vec<Attribute>, vis: Visibility) -> Result<Self> {
        let _ = input.parse::<Token![struct]>()?;
        let name = input.parse()?;
        let generics: Generics = input.parse()?;
        let _ = input.parse::<keywords::variants>()?;

        //We actually don't need these to reconstruct the types, beause they effectively just serve as
        // pre-declarations, for the information in the variants' sub_type_bindings
        let _runtime_generic_types = extract_runtime_generic_types(input.parse::<Generics>()?)?;

        let variants_group_contents: ParseBuffer;
        let _brace_token = syn::braced!(variants_group_contents in input);
        let sub_types = Self::parse_sub_types_from_struct_variants_clause(variants_group_contents)?;
        let cases = Self::build_variants_from_sub_types(&name, &generics, &sub_types)?;

        let struct_fields_content: ParseBuffer;
        let _brace_token = syn::braced!(struct_fields_content in input);
        let struct_fields = struct_fields_content.parse_terminated(Field::parse_named, Token![,])?
            .into_iter().collect();

        Ok(Self {
            attrs,
            vis,
            name,
            generics,
            cases,
            sub_types,
            struct_fields,
        })
    }

    fn parse_sub_types_from_struct_variants_clause(input: ParseBuffer) -> Result<Vec<SubType>> {
        let mut sub_types = vec![];

        while !input.is_empty() {
            //parse variant name identifier
            let variant_name = input.parse::<Ident>()?;

            //parse bindings block
            let bindings_group_contents: ParseBuffer;
            let _paren_token = syn::parenthesized!(bindings_group_contents in input);
            let bindings = Self::parse_bindings_group(bindings_group_contents)?;

            sub_types.push(SubType{variant_name, bindings});

            //Expect ','
            let _ = input.parse::<Option<Token![,]>>();
        }
        Ok(sub_types)
    }

    fn parse_bindings_group(input: ParseBuffer) -> Result<Vec<(Ident, Type)>> {
        let mut bindings = vec![];
        while !input.is_empty() {
            let key = input.parse::<Ident>()?;
            let _ = input.parse::<Token![=]>()?;
            let binding_type = input.parse::<Type>()?;
            let _ = input.parse::<Option<Token![,]>>();
            bindings.push((key, binding_type));
        }
        Ok(bindings)
    }

    fn build_variants_from_sub_types(name: &Ident, generics: &Generics, sub_types: &Vec<SubType>) -> Result<Vec<Variant>> {
        let (_impl_generics, type_generics, _where_clause) = generics.split_for_impl();
        let mut cases = vec![];

        for sub_type in sub_types {
            let variant_name = &sub_type.variant_name;
            let sub_type_name = sub_type.struct_type_ident(name);

            let mut variant: Variant = parse(quote!{ #variant_name(#sub_type_name #type_generics) }.into())?;
            let sub_type = type_from_fields_mut(&mut variant.fields);
            cannonicalize_type_path(sub_type);
            cases.push(variant);
        }

        Ok(cases)
    }

    fn parse(input: ParseStream, attrs: Vec<Attribute>) -> Result<Self> {
        let vis = input.parse()?;

        if input.peek(Token![type]) {
            SummumType::parse_haskell_style(input, attrs, vis)
        } else if input.peek(Token![enum]) {
            SummumType::parse_enum_style(input, attrs, vis)
        } else if input.peek(Token![struct]) {
            SummumType::parse_struct(input, attrs, vis)
        }else {
            input.step(|cursor| {
                Err(cursor.error(format!("expected `enum`, `struct`, `type`, or `impl`")))
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
            sub_types,
            struct_fields,
        } = self;

        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
        let top_enum_type: Type = parse_quote! { #name #type_generics };

        // render `impl From<VariantT> for SumT`
        let cases_tokens = cases.iter().map(|variant| quote! {
            #variant
        }).collect::<Vec<_>>();
        let from_impls = cases.iter().map(|variant| {
            let ident = &variant.ident;
            let sub_type = type_from_fields(&variant.fields);

            quote_spanned! {variant.span() =>
                impl #impl_generics From<#sub_type> for #top_enum_type #where_clause {
                    fn from(val: #sub_type) -> Self {
                        #name::#ident(val)
                    }
                }
            }
        }).collect::<Vec<_>>();

        // render `impl TryFrom<SumT> for VariantT`
        let generic_params = type_params_from_generics(&generics);
        let try_from_impls = cases.iter().map(|variant| {
            let ident = &variant.ident;
            let sub_type = type_from_fields(&variant.fields);
            if !detect_uncovered_type(&generic_params[..], &sub_type) {
                quote! {
                    impl #impl_generics core::convert::TryFrom<#top_enum_type> for #sub_type #where_clause {
                        type Error = ();
                        fn try_from(val: #top_enum_type) -> Result<Self, Self::Error> {
                            match val{#name::#ident(val)=>Ok(val), _=>Err(())}
                        }
                    }
                }
            } else {
                quote!{}
            }
        }).collect::<Vec<_>>();

        // render `SumT::variants() and SumT::variant_name()`
        let variants_strs = cases.iter().map(|variant| {
            let ident_string = &variant.ident.to_string();
            quote_spanned! {variant.span() =>
                #ident_string
            }
        }).collect::<Vec<_>>();
        let variant_name_branches = cases.iter().map(|variant| {
            let ident = &variant.ident;
            let ident_string = ident.to_string();
            quote_spanned! {variant.span() =>
                Self::#ident(_) => #ident_string
            }
        }).collect::<Vec<_>>();
        let variants_impl = quote!{
            #[allow(dead_code)]
            impl #impl_generics #top_enum_type #where_clause {
                pub const fn variants() -> &'static[&'static str] {
                    &[#(#variants_strs),* ]
                }
                pub fn variant_name(&self) -> &'static str {
                    match self{
                        #(#variant_name_branches),*
                    }
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

        // render individual variant accessor methods
        let accessor_impls = cases.iter().map(|variant| {
            let ident = &variant.ident;
            let sub_type = type_from_fields(&variant.fields);

            let ident_string = ident.to_string();
            let is_fn_name = Ident::new(&snake_name("is", &ident_string), variant.ident.span());
            let try_as_fn_name = Ident::new(&snake_name("try_as", &ident_string), variant.ident.span());
            let as_fn_name_str = snake_name("as", &ident_string);
            let as_fn_name = Ident::new(&as_fn_name_str, variant.ident.span());
            let try_as_mut_fn_name = Ident::new(&snake_name("try_as_mut", &ident_string), variant.ident.span());
            let as_mut_fn_name_str = snake_name("as_mut", &ident_string);
            let as_mut_fn_name = Ident::new(&as_mut_fn_name_str, variant.ident.span());
            let try_into_fn_name = Ident::new(&snake_name("try_into", &ident_string), variant.ident.span());
            let into_fn_name_str = snake_name("into", &ident_string);
            let into_fn_name = Ident::new(&into_fn_name_str, variant.ident.span());

            let error_msg = format!("invalid downcast: {name}::{{}} expecting {ident_string} found {{}}");
            quote_spanned! {variant.span() =>
                pub fn #is_fn_name(&self) -> bool {
                    match self{Self::#ident(_)=>true, _=>false}
                }
                pub fn #try_as_fn_name(&self) -> Option<&#sub_type> {
                    match self{Self::#ident(val)=>Some(val), _=>None}
                }
                pub fn #as_fn_name(&self) -> &#sub_type {
                    self.#try_as_fn_name().unwrap_or_else(|| panic!(#error_msg, #as_fn_name_str, self.variant_name()))
                }
                pub fn #try_as_mut_fn_name(&mut self) -> Option<&mut #sub_type> {
                    match self{Self::#ident(val)=>Some(val), _=>None}
                }
                pub fn #as_mut_fn_name(&mut self) -> &mut #sub_type {
                    let variant_name = self.variant_name();
                    self.#try_as_mut_fn_name().unwrap_or_else(|| panic!(#error_msg, #as_mut_fn_name_str, variant_name))
                }
                pub fn #try_into_fn_name(self) -> core::result::Result<#sub_type, Self> {
                    match self{Self::#ident(val)=>Ok(val), _=>Err(self)}
                }
                pub fn #into_fn_name(self) -> #sub_type {
                    self.#try_into_fn_name().unwrap_or_else(|t| panic!(#error_msg, #into_fn_name_str, t.variant_name()))
                }
            }
        }).collect::<Vec<_>>();
        let accessors_impl = quote!{
            #[allow(dead_code)]
            impl #impl_generics #top_enum_type #where_clause {
                #(#accessor_impls)*
            }
        };

        // //TODO: re-enable this feature when https://github.com/rust-lang/rust/issues/8995 is available in stable
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
        //     impl #impl_generics #top_enum_type #where_clause {
        //         #(#variant_type_aliases)*
        //     }
        // };

        //Render the sub-type structs
        let sub_types_vec = sub_types.iter().map(|sub_type| {
            let variant_name = sub_type.struct_type_ident(name);
            let sub_type_fields = remap_sub_type_fields(&struct_fields[..], &sub_type.bindings, &top_enum_type);
            quote_spanned! {variant_name.span() =>
                #(#attrs)*
                #vis struct #variant_name #type_generics #where_clause {
                    #(#sub_type_fields),*
                }
            }
        }).collect::<Vec<_>>();

        //Top-level renderer that produces the output
        quote! {
            #[allow(dead_code)]
            #(#attrs)*
            #vis enum #top_enum_type #where_clause {
                #(#cases_tokens),*
            }

            #(#sub_types_vec)*

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
                let mut variant_blocks = vec![];
                for variant in item_type.cases.iter() {
                    let ident = &variant.ident;
                    let ident_string = ident.to_string();
                    let variant_t_name = format!("{}T", ident_string);

                    let sub_type = type_from_fields(&variant.fields);
                    let sub_type_string = quote!{ #sub_type }.to_string();

                    //Swap all the occurance of `self`, etc. in the block
                    let block_tokenstream = replace_idents(item.block.to_token_stream(), &[
                        ("self", "_summum_self"),
                        ("super", "self"),
                        ("VariantT", &variant_t_name),
                        ("InnerT", &sub_type_string),
                    ], &[
                        ("_inner_var", &|base| snake_name(base, &ident_string))
                    ]);

                    //Handle the "exclude" and "restrict" virtual control macros in the function body
                    let block_tokenstream = match handle_inner_macros(block_tokenstream.into(), &ident_string) {
                        Ok(block_tokenstream) => block_tokenstream,
                        Err(err) => {return err.into();}
                    };

                    let block: Block = parse(quote_spanned!{item.block.span() => { #block_tokenstream } }.into()).expect("Error composing sub-block");
                    variant_blocks.push(block);
                }

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
    //Question: should we be using TypeGenerics::as_turbofish() in here somewhere??
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
    let type_stream = quote!{ #item_type };
    let ident: TypeIdentParseHelper = parse2(type_stream)?;
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

const MACRO_IDENT_LIST: &'static[&'static str] = &["summum_exclude", "summum_restrict", "summum_variant_name"];

//Implement the "summum_exclude!" and "summum_restrict!" virtual macros
fn handle_inner_macros(input: proc_macro2::TokenStream, branch_ident: &str) -> core::result::Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
    let mut new_stream = proc_macro2::TokenStream::new();

    // Ugh.  I wish I could just use the `parse` functionality in the `syn` crate, but I need
    // to parameterize the types I'm trying to extract at runtime.  And that doesn't seem to be
    // a supported use case.  This comment applies to many functions that look redundant

    let mut input_iter = input.into_iter().peekable();
    while let Some(item) = input_iter.next() {
        match item {
            TokenTree::Ident(ident) => {
                let ident_string = ident.to_string();
                if let Some(macro_ident_str) = MACRO_IDENT_LIST.iter().find_map(|key| {
                    if key == &ident_string {
                        Some(*key)
                    } else {
                        None
                    }
                }) {
                    parse_punct(input_iter.next(), '!')?;
                    let next_item = input_iter.next();
                    let next_span = next_item.span();
                    let macro_args = if let Some(TokenTree::Group(macro_args_group)) = next_item {
                        let args_group_stream = macro_args_group.stream();
                        let macro_args_punct: Punctuated::<Ident, Token![,]> = parse_quote!( #args_group_stream );
                        let macro_args: Vec<String> = macro_args_punct.into_iter().map(|ident| ident.to_string()).collect();
                        macro_args
                    } else {
                        return Err(quote_spanned! {next_span => compile_error!("Expecting tuple for macro args"); }.into());
                    };
                    if parse_punct(input_iter.peek(), ';').is_ok() {
                        let _ = input_iter.next();
                    }

                    match macro_ident_str {
                        "summum_exclude" | "summum_restrict" => {
                            let is_exclude = ident_string == "summum_exclude";
                            let branch_in_list = macro_args.iter().find(|arg| arg.as_str() == branch_ident).is_some();

                            if (is_exclude && branch_in_list) || (!is_exclude && !branch_in_list) {
                                let unreachable_message = &format!("internal error: encountered {ident_string} on {branch_ident} branch");
                                let panic_tokens = quote_spanned!{ident.span() =>
                                    {
                                        #new_stream
                                        panic!(#unreachable_message);
                                        // #[allow(unreachable_code)]
                                    }
                                };
                                return Ok(panic_tokens);
                            }
                        },
                        "summum_variant_name" => {
                            let new_tokens = quote_spanned!{ident.span() =>
                                #branch_ident
                            };
                            new_stream.extend(new_tokens);
                        },
                        _ => unreachable!()
                    }
                } else {
                    new_stream.extend([TokenTree::Ident(ident)]);
                }
            },
            TokenTree::Group(group) => {
                let new_group_stream = handle_inner_macros(group.stream(), branch_ident)?;
                let mut new_group = Group::new(group.delimiter(), new_group_stream);
                new_group.set_span(group.span());
                new_stream.extend([TokenTree::Group(new_group)]);
            },
            _ => {new_stream.extend([item]);}
        }
    }
    Ok(new_stream)
}

fn parse_punct<T: core::borrow::Borrow<TokenTree>>(item: Option<T>, the_char: char) -> core::result::Result<(), proc_macro2::TokenStream> {
    let item_ref = item.as_ref().map(|i| i.borrow());
    let span = item_ref.span();
    if let Some(TokenTree::Punct(p)) = item_ref {
        if p.as_char() == the_char {
            return Ok(());
        }
    }
    let err_string = format!("expecting {the_char}");
    return Err(quote_spanned! {span => compile_error!(#err_string); }.into());
}

fn sig_contains_self_arg(sig: &Signature) -> bool {
    if let Some(first_arg) = sig.inputs.first() {
        if let FnArg::Receiver(_rcvr) = first_arg {
            return true;
        }
    }
    false
}

fn extract_runtime_generic_types(generics: Generics) -> Result<Vec<Ident>> {
    let (_impl_generics, _type_generics, where_clause) = generics.split_for_impl();
    if let Some(where_clause) = where_clause {
        return Err(Error::new(where_clause.span(), "where clause illegal for runtime generics"));
    }
    let type_params = type_params_from_generics(&generics);
    let results = type_params.into_iter().map(|type_param| {
        type_param.ident.clone()
    }).collect();

    Ok(results)
}

/// Replaces types inside the fields of a struct using a bindings table.
/// Additionally, `InnerT` maps to `Self`, and `Self` maps onto the parent type
fn remap_sub_type_fields(src_fields: &[Field], bindings: &[(Ident, Type)], parent_enum_type: &Type) -> Vec<Field> {
    //QUESTION FOR FUTURE: should it seems silly to convert all these types into strings, only to
    // convert them back into types.  But I don't want to touch the machinery of replace_idents
    // right now, and also a string is a reasonable lowest-common-denominator
    let parent_enum_string = quote!{ #parent_enum_type }.to_string();
    let stringified_bindings: Vec<(String, String)> = bindings.into_iter()
            .map(|(key, ty)| (key.to_string(), quote!{ #ty }.to_string())).collect();
    let pairs: Vec<(&str, &str)> = stringified_bindings.iter()
            .map(|(key, ty)| (key.as_str(), ty.as_str()))
            .map(|(key, ty)| { match ty {
                "Self" => (key, parent_enum_string.as_str()), //Remapping to Self should actually remap to the parent enum
                "InnerT" => (key, parent_enum_string.as_str()), //Remapping to InnerT should actually remap to the Self
                _ => (key, ty)
            }})
            .chain([("Self", parent_enum_string.as_str()), ("InnerT", "Self")].into_iter()).collect();

    let mut new_fields = vec![];
    for field in src_fields {
        let field_type = &field.ty;
        let field_tokens = quote!{ #field_type };
        let new_tokens = replace_idents(field_tokens, &pairs, &[]);
        let mut new_field = field.clone();
        new_field.ty = parse2(new_tokens).unwrap();
        new_fields.push(new_field);
    }
    new_fields
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


//TODO - plan for adding structs 
// * allow structs to be defined.
//    - Include a separate block to define:
//    - variant names
//    - additional generic types
//    - mappings for each generic to a concrete type or higher-level generic
//
//  * struct def needs to spin out a variant struct for each
//  * also a unifying enum
//  * When manifesting the structs, "InnerT" maps to `Self`, and Self maps to the enum
//
// * Need to inject phantom_data into subtypes that don't use all the type_generics.
//   BETTER IDEA: strip the type-generics for the sub-type.  I should do this after the re-mapping, so I can track which vars are ultimately used, since the re-mapping can map to generic vars
//
// * place struct method impls inside their own types's impl
//   - and call to that impl from the outer enum impl
//
//
//
//README:
// - "Enums as Sum-Types"
// - "Runtime-Selected Generic Types in Structs"
//