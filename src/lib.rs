
use proc_macro::TokenStream;
use quote::quote;
use heck::{AsUpperCamelCase, AsSnakeCase};
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, Attribute, ItemEnum, Fields, Variant, GenericParam, TypeParam, Ident, Token, Type, Generics, Visibility, parse};
use syn::spanned::Spanned;

struct TypeItem {
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    generics: Generics,
    cases: Vec<Variant>,
}

impl TypeItem {
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
}

impl Parse for TypeItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;

        if input.peek(Token![type]) {
            TypeItem::parse_haskell_style(input, attrs, vis)
        } else if input.peek(Token![enum]) {
            TypeItem::parse_enum_style(input, attrs, vis)
        } else {
            input.step(|cursor| {
                Err(cursor.error(format!("expected `enum` or `type`")))
            })
        }
    }
}

struct Args {
    superset: Option<Ident>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(if let Ok(_) = input.parse::<Token![super]>() {
            let _ = input.parse::<Token![=]>()?;
            Self {
                superset: input.parse()?,
            }
        } else {
            Self { superset: None }
        })
    }
}

// /// Create an enum that contains a case for all given types
// ///
// /// # Examples
// /// By default, enum cases are named after their contained type. To pick a different name, you can use a type alias:
// /// ```rust
// /// use typeunion::type_union;
// ///
// /// type Int = i64;
// ///
// /// #[type_union]
// /// #[derive(Debug, PartialEq)]
// /// type Union = String + Int;
// ///
// /// // `From` is derived automatically for all cases
// /// let my_string: Union = "Hello World!".to_string().into();
// /// let my_enum_case = Union::String("Hello World!".to_string());
// /// assert_eq!(my_string, my_enum_case);
// /// ```
// ///
// /// Typeunions can declare a super set, that they should be convertible to:
// /// ```rust
// /// use typeunion::type_union;
// /// use std::sync::Arc;
// ///
// /// type BoxedStr = Box<str>;
// /// type ArcStr = Arc<str>;
// ///
// /// #[type_union(super = SomeString)]
// /// type UniqueString = String + BoxedStr;
// ///
// /// #[type_union]
// /// #[derive(Debug, PartialEq)]
// /// type SomeString = String + BoxedStr + ArcStr;
// ///
// /// let a: UniqueString = "a".to_string().into();
// /// let b: SomeString = "a".to_string().into();
// /// let a_lower: SomeString = a.into();
// /// assert_eq!(a_lower, b);
// /// ```
// #[proc_macro_attribute]
// pub fn type_union(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let Args { superset } = parse_macro_input!(attr as Args);

//     println!("GOAT First thing");

//     let TypeItem {
//         attrs,
//         vis,
//         name,
//         generics,
//         // cases, goat
//     } = parse_macro_input!(item as TypeItem);

//     println!("GOAT finished parse");

//     //let cases = cases.into_iter().map(|ident| ident).collect::<Vec<_>>();

//     // let impls = if let Some(superset) = superset {
//     //     quote! {
//     //         impl From<#name> for #superset {
//     //             fn from(value: #name) -> Self {
//     //                 match value {
//     //                     #(#name::#cases(case) => #superset::#cases(case)),*
//     //                 }
//     //             }
//     //         }
//     //     }
//     // } else {
//     //     quote!()
//     // };

//     // quote! {
//     //     #(#attrs)*
//     //     #vis enum #name {
//     //         #(#cases(#cases)),*
//     //     }

//     //     #impls

//     //     #(
//     //         impl From<#cases> for #name {
//     //             fn from(value: #cases) -> Self {
//     //                 #name::#cases(value)
//     //             }
//     //         }
//     //     )*
//     // }
//     // .into()

//     //GOAT
//     quote!{
//         #vis struct #name;
//     }.into()
// }

#[proc_macro]
pub fn summum(input: TokenStream) -> TokenStream {

    let TypeItem {
        attrs,
        vis,
        name,
        generics,
        cases,
    } = parse_macro_input!(input as TypeItem);

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

    // let impls = if let Some(superset) = superset {
    //     quote! {
    //         impl From<#name> for #superset {
    //             fn from(value: #name) -> Self {
    //                 match value {
    //                     #(#name::#cases(case) => #superset::#cases(case)),*
    //                 }
    //             }
    //         }
    //     }
    // } else {
    //     quote!()
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
    }
    .into()
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
//GOAT, attribute or something so From<> and TryFrom<> impl can be disabled to avoid conflict when two variants have the same type
