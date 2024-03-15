# summum-types
A sum-type macro crate with all the conversion, accessors and support for abstract methods across variants

## Summary

This crate strives to provide dynamic runtime-resolving types on top of Rust’s static compile-time types, with full support for generics, lifetimes, visibility, etc.

The `summum` macro allows for easy declaration of sum-types that:
- come with all the traits and methods you'd expect for conversion and access
- allow generic method implementation across all variants
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

You can also use `Self` as a local type alias that expands to the variant type.  If you're returning `Self`, you'll need to remember to use the `.into()` conversion.  Like this:

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

Yes, all abstract methods need `&self` to know which variant type to use.  You can also implement ordinary methods on the type *outside* the `summum` invocation.

### Polymorphism

One of the uses for sum-type enums is to fill a similar role to `dyn` trait objects in polymorphic method dispatch.  Sum-type enums provide different design constraints, such as being `Sized` and not requiring object safety.  Unlike the [Any trait](https://doc.rust-lang.org/std/any/index.html) in particular, summum types provide a method to recover ownership of the original type, and allowing internal lifetimes (no `'static` bound).

Sum-types are certainly not a replacement for dynamic dispatch in every case, but hopefully they will be another tool to reach for when it's convenient.

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

* `impl` blocks must be in the same `summum!` macro invocation where the types are defined.  This is the primary reason `summum` is not an attrib macro.  The limitation is due to [this issue](https://github.com/rust-lang/rust/issues/44034) and the work-around¹ is likely more fragile and a worse experience than just keeping the impls together.

¹It's possible to implement the macro expansion in two passes where the second macro is created on the fly, folding in information from the source code.  But it's a bit of a Rube Goldberg machine.

### Future Work

#### Abstract Method Declarations

In the vein of polymorphic method dispatch, I'd like to support "trait style" method declarations without a body.  It's just syntactic sugar over the existing abstract `impl` dispatch, but it would make the declaration of an abstract sum-type with methods look much cleaner.

#### Inheritance and SuperTypes

[typeunion by Antonius Naumann](https://github.com/antoniusnaumann/typeunion) includes 
inheritance and that comes with automatic conversion between some sum-types that share variants in common.  Any runtime-resolving type system ought to have that kind of thing.

#### Future Plan for Accessors

I'd like to implement generic accessors, along the lines of: `pub fn try_into_sub<T>(self) -> Option<T>`, for example.  This would eliminate the annoyance of remembering/ guessing what identifier is assigned to a particular variant.  Unfortunately that seems to be blcoked on [this issue](https://github.com/rust-lang/rust/issues/20041) for the time being.

## Acknowledgement & Other Options

Several other union type / sum type crates exist, and they might be better for your use case

- [typeunion by Antonius Naumann](https://github.com/antoniusnaumann/typeunion) is great if you want something lightweight, and it has an sweet supertype feature
- [sum_type by Michael F. Bryan](https://github.com/Michael-F-Bryan/sum_type) is `no-std` and manages to do everything with declarative macros
- [typesum by Natasha England-Elbro](https://github.com/0x00002a/typesum) is awesome for the control it gives you over the generated output and the way it supports overlapping base types beautifully.

Each of them have things they do uniquely well and I took inspiration from all of them.
