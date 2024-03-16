# summum-types
A sum-type macro crate with all the conversion, accessors and support for abstract methods across variants, and interoperability between sum-types

## Summary

This crate strives to provide dynamic runtime-resolving types on top of Rust’s static compile-time types, with full support for generics, lifetimes, visibility, etc.

The `summum` macro allows for easy declaration of sum-types that:
- come with all the traits and methods you'd expect for conversion and access
- allow generic method implementation across all variants
- support interoperability across multiple types via shared variant names
<!-- - support abstract interfaces to delegate to sub-type methods -->

### Motivation

> Rust's `enum`s are already sum types, so why do I need this crate?

Lots and lots of boilerplate written for you.

I realized I needed something like this when I tried to implement a recursive type definition.  [Rust's static type system could not represent the type](https://users.rust-lang.org/t/recursive-generic-type-parameters-full-featured-union-types/108114) that I needed without imposing a finite recursion depth.  But using an `enum` doubled the size of my implementation because monomorphization across the variants wasn't supported.

### Summum??

It'a just a dumb pun.  It means "highest" in Latin.  No connection whatsoever to the [pyramid people in Utah](https://en.wikipedia.org/wiki/Summum).

## Usage

Define a sum type inside the `summum` macro, but otherwise it's just like any other enum:
```rust
summum!{
    #[derive(Debug, Clone)]
    enum SliceOrPie<'a, T> {
        Slice(&'a [T]),
        Vec(Vec<T>),
    }
}
```

And you automatically get all the accessors you'd want¹:
- [From](https://doc.rust-lang.org/std/convert/trait.From.html) `impl` to create the sum type from any of each of its variants
- [TryFrom](https://doc.rust-lang.org/std/convert/trait.TryFrom.html) `impl`, to convert the sum type back to any of its variants.²
- `pub fn is_*t*(&self) -> bool`
- `pub fn try_as_*t*(&self) -> Option<&T>`
- `pub fn as_*t*(&self) -> &T`
- `pub fn try_as_mut_*t*(&mut self) -> Option<&mut T>`
- `pub fn as_mut_*t*(&mut self) -> &mut T`
- `pub fn try_into_*t*(self) -> Option<T>`
- `pub fn into_*t*(self) -> T`
- `pub fn SumT::variants() -> &[&str]`

**Note**: `*t*` is a lower_snake_case rendering of the variant identifier

¹If you want more accessors (or features in general), please email me  
²Except where the variant type would be an "uncovered" generic as described [here](https://doc.rust-lang.org/error_codes/E0210.html)  

### Generic method impl dispatch

You can also add method `impl` blocks, to implement functionality shared across every variant within your sum-type.  This expands to a match statement on `&self`, where `&self` is remapped to a local variable if the inner variant type.  For example:

```rust
summum!{
    #[derive(Debug, Clone)]
    enum SliceOrPie<'a, T> {
        Slice(&'a [T]),
        Vec(Vec<T>),
    }

    impl<'a, T> SliceOrPie<'a, T> {
        fn get(&self, idx: usize) -> Option<&T> {
            self.get(idx)
        }
    }
}
```

You can also use `Self` as a local type alias that expands to the variant type.  Also `InnerT` is an alias to the concrete type of the variant being expanded.  If your method is returning `Self`, you'll need to remember to use the `.into()` conversion to get back to the sum-type.  Like this:

```rust
summum!{
    enum Num {
        F64(f64),
        I64(i64),
    }

    impl Num {
        fn max_of_type(&self) -> Self {
            Self::MAX.into()
        }
    }
}
```

Yes, all abstract methods need `self` to know which variant type to use.  You can also use a *Variant Specialized Method* (keep reading) for constructors and other places where you don't want a `self` argument.

Of course uou can also implement ordinary methods on the sub-type *outside* the `summum` invocation, where these rules don't apply.

### Variant Specialized Methods

Sometimes you need to generate an explicit method for each variant.  `summum` has you covered.  Just end a method name with `"_inner_var"` and it will be replaced by a separate method for each variant.  For example, the code below will lead to the generation of a the `max_f64` and `max_i64` methods.

```rust
summum!{
    enum Num {
        F64(f64),
        I64(i64),
    }

    impl Num {
        fn max_inner_var() -> Self {
            Self::MAX.into()
        }
    }
}
```

You can also pass `self` as an argument to variant-specialized methods.  Be warned, however, if the inner type of `self` doesn't agree with the method variant then the method will panic!

Within a variant-specialized method, you can use `InnerT` in the function signature, for both arguments and the method return type.  For example:

```rust
    //Within the `summum` invocation above...

    impl Num {
        fn multiply_add_one_inner_var(&self, multiplier: InnerT) -> InnerT {
            *self * multiplier + 1 as InnerT
        }
    }
```

### Polymorphism

One of the uses for sum-type enums is to fill a similar role to `dyn` trait objects in polymorphic method dispatch.  Sum-type enums provide different design constraints, such as being `Sized` and not requiring object safety.  Unlike the [Any trait](https://doc.rust-lang.org/std/any/index.html) in particular, summum types provide a method to recover ownership of the original type, and allowing internal lifetimes (no `'static` bound).

Sum-types are certainly not a replacement for dynamic dispatch in every case, but hopefully they will be another tool to reach for when it's convenient.

### Variant Substitution in Method Calls for Interoperation Across Types

Consider multiple types that interact with each other like in the example below.  Sometimes we need to interact with a related type in a way that depends on which variant we're generating.  In those cases, we can call the synthesized variant-specific functions of other types, as long as the variant names of the `impl` type are a superset of the type being called.

```rust
summum!{
    enum Num {
        F64(f64),
        I64(i64),
    }

    enum NumVec {
        F64(Vec<f64>),
        I64(Vec<i64>),
    }

    impl NumVec {
        fn push(&mut self, item: Num) {
            // This will be replaced with either `into_f64` or `into_i64` depending
            // on the variant branch being generated
            let val = item.into_inner_var();
            self.push(val);
        }
    }
}
```

### Bonus Syntax: Haskell / TypeScript Style

If you're into the whole brevity thing, you can write: 
```rust
summum!{
    type Num = f64 | i64;
}
```

You can use the `as` keyword to rename variants using this syntax:
```rust
summum!{
    type VecOrVRef<'a, V> = &'a Vec<V> as Vec | 
                            &'a V as V;
}
```

### Limitations

* This macro can generate *a lot* of code, most of which will be eliminated as dead.  If overused, this might result in degraded build times.  Also summum sum-types are probably not appropriate for exposing in a public API, but YMMV.

* `impl` blocks must be in the same `summum!` macro invocation where the types are defined.  This is the primary reason `summum` is not an attrib macro.  The limitation is due to [this issue](https://github.com/rust-lang/rust/issues/44034) and the work-around¹ is likely more fragile and a worse experience than just keeping the impls together.

* Each inner type should occur only once within a sum-type.  The purpose of this crate is runtime dynamism over multiple types.  If you want to multiple variants backed by the same type, then you could define type aliases.  Or you try [typesum by Natasha England-Elbro](https://github.com/0x00002a/typesum).

¹It's possible to implement the macro expansion in two passes where the second macro is created on the fly, folding in information from the source code.  But it's a bit of a Rube Goldberg machine.

### Future Work

#### Abstract Method Declarations

In the vein of polymorphic method dispatch, I'd like to support "trait style" method declarations without a body.  It's just syntactic sugar over the existing abstract `impl` dispatch, but it would make the declaration of an abstract sum-type with methods look much cleaner.

#### Associated Types for Each Variant

I'd like to add support for accessing the type of each variant through an associated type alias.  So relative to the `Num` example above, the declaration would also include `type F64T = f64`.  What's the point of that?  By itself, not much.  But combine that with the ability for another type's implementation to reference this type via shared variants, using the `::VariantT` type alias, and you can do this:

```rust
summum!{
    enum Num {
        F64(f64),
        I64(i64),
    }

    enum NumVec {
        F64(Vec<f64>),
        I64(Vec<i64>),
    }

    impl NumVec {
        fn push_inner_var(&mut self, item: Num::VariantT) {
            self.push(item)
        }
        fn get_or_default(&self, idx: usize) -> Num {
            self.get(idx).cloned().unwrap_or_else(|| Num::VariantT::default() ).into()
        }
    }
}
```

This feature is currently disabled on account of [this issue](https://github.com/rust-lang/rust/issues/8995).  Hopefully this will reach stable soon and I can re-enable it.

#### Future Plan for Accessors

I'd like to implement generic accessors, along the lines of: `pub fn try_into_sub<T>(self) -> Option<T>`, for example.  This would eliminate the annoyance of remembering/ guessing what identifier is assigned to a particular variant.  Unfortunately that seems to be blcoked on [this issue](https://github.com/rust-lang/rust/issues/20041) for the time being.

## Acknowledgement & Other Options

Several other union type / sum type crates exist, and one of them might be better for your use case.  Each has things they do uniquely well and I took inspiration from all of them.

- [typeunion by Antonius Naumann](https://github.com/antoniusnaumann/typeunion) is great if you want something lightweight, and it has an sweet supertype feature for automatic conversions between types with variants in common.
- [sum_type by Michael F. Bryan](https://github.com/Michael-F-Bryan/sum_type) is `no-std` and manages to do everything with declarative macros.  Also it supports downcasting for variant types that can implement the `Any` trait.
- [typesum by Natasha England-Elbro](https://github.com/0x00002a/typesum) is awesome for the control it gives you over the generated output and the way it supports overlapping base types beautifully.  It'll cope much better if you plan to have a silly number of variants.

Finally, thank you for looking at this crate.  If you have ideas and/or feedback, please let me know, either via email or with a GitHub issue.