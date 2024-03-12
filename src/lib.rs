use proc_macro::TokenStream;
use quote::quote;
use heck::AsUpperCamelCase;
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, Attribute, ItemEnum, Variant, Ident, Token, Type, Generics, Visibility, parse};
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

    println!("GOAT finished parse");

    let cases_tokens = cases.into_iter().map(|variant| {


        println!("GOAT {:?}", variant.ident);
        // quote! { #item_ident(#item_type) }

        quote! { #variant }

    }).collect::<Vec<_>>();

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
        #(#attrs)*
        #vis enum #name #generics {
            #(#cases_tokens),*
        }

        //#impls

        // #(
        //     impl From<#cases> for #name {
        //         fn from(value: #cases) -> Self {
        //             #name::#cases(value)
        //         }
        //     }
        // )*
    }
    .into()

    // quote!{
    //     #vis struct #name;
    // }.into()
}


fn ident_from_type(item_type: &Type) -> Ident {
    let item_ident = quote!{ #item_type }.to_string();
    let item_ident = AsUpperCamelCase(item_ident).to_string();
    Ident::new(&item_ident, item_type.span())
}
