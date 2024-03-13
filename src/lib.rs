
use proc_macro::TokenStream;
use quote::quote;
use heck::{AsUpperCamelCase, AsSnakeCase};
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, Attribute, ItemEnum, Fields, Variant, GenericParam, TypeParam, Ident, Token, Type, Generics, Visibility, parse};
use syn::spanned::Spanned;

use std::collections::HashMap;

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
                ident_from_type(&item_type)
            };

            let variant: Variant = parse(quote!{ #item_ident(#item_type) }.into())?;
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
        let cases = enum_block.variants.into_iter().collect();

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

            quote! {
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
            quote! {
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

            let is_fn_name = snake_ident("is", &variant.ident);
            let try_borrow_fn_name = snake_ident("try_borrow", &variant.ident);
            let borrow_fn_name = snake_ident("borrow", &variant.ident);
            let try_borrow_mut_fn_name = snake_ident("try_borrow_mut", &variant.ident);
            let borrow_mut_fn_name = snake_ident("borrow_mut", &variant.ident);
            let try_into_fn_name = snake_ident("try_into", &variant.ident);
            let into_fn_name = snake_ident("into", &variant.ident);

            quote! {
                pub fn #is_fn_name(&self) -> bool {
                    match self{Self::#ident(_)=>true, _=>false}
                }
                pub fn #try_borrow_fn_name(&self) -> Option<&#sub_type> {
                    match self{Self::#ident(val)=>Some(val), _=>None}
                }
                pub fn #borrow_fn_name(&self) -> &#sub_type {
                    self.#try_borrow_fn_name().unwrap()
                }
                pub fn #try_borrow_mut_fn_name(&mut self) -> Option<&mut #sub_type> {
                    match self{Self::#ident(val)=>Some(val), _=>None}
                }
                pub fn #borrow_mut_fn_name(&mut self) -> &mut #sub_type {
                    self.#try_borrow_mut_fn_name().unwrap()
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
            impl #generics #name #generics {
                #(#accessor_impls)*
            }
        };

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

            // #guide_type
        }
        .into()
    }
}

struct SummumImpl {
    attrs: Vec<Attribute>,
    name: Ident,
    impl_generics: Generics,
    type_generics: Generics,
}

impl SummumImpl {
    fn parse(input: ParseStream, attrs: Vec<Attribute>) -> Result<Self> {
        let _ = input.parse::<Token![impl]>()?;
        let impl_generics: Generics = input.parse()?;
        let name = input.parse()?;
        let type_generics: Generics = input.parse()?;

        Ok(Self {
            attrs,
            name,
            impl_generics,
            type_generics,
        })
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

#[proc_macro]
pub fn summum(input: TokenStream) -> TokenStream {
    let mut new_stream = TokenStream::new();
    let items: SummumItems = parse_macro_input!(input as SummumItems);

    for item in items.types.values() {
        new_stream.extend(item.render());
    }

    new_stream
}


fn ident_from_type(item_type: &Type) -> Ident {
    let item_ident = quote!{ #item_type }.to_string();
    let item_ident = AsUpperCamelCase(item_ident).to_string();
    Ident::new(&item_ident, item_type.span())
}

fn snake_ident(base: &str, ident: &Ident) -> Ident {
    let ident_string = format!("{base}_{}", AsSnakeCase(ident.to_string()));
    Ident::new(&ident_string, ident.span())
}

fn type_from_fields(fields: &Fields) -> &Type {
    if let Fields::Unnamed(field) = fields {
        &field.unnamed.first().unwrap().ty
    } else {panic!()}
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

//GOAT, remember to generate an example so docs will be built
//GOAT, attribute so From<> and TryFrom<> impl can be disabled to avoid conflict when two variants have the same type
