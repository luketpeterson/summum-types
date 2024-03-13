# summum-types
A sum-type macro crate with all the features

## Summary

This crate strives to provide dynamic runtime-resolving types on top of Rust’s static compile-time types.

To that end, the `summum` macro allows for easy declaration of sum-types that:
- come with all the traits and methods you'd expect for conversion and access
- allow generic method implementation across all variants
- support abstract interfaces to delegate to sub-type methods

### Motivation

> Rust's `enum`s are already sum types, so why do I need this crate?

Lots and lots of boilerplate written for you.

I realized I needed something like this when I tried to implement a recursive type definition.  [Rust's static type system could not represent the type](https://users.rust-lang.org/t/recursive-generic-type-parameters-full-featured-union-types/108114) that I needed without imposing a finite recursion depth.  But using an `enum` doubled the size of my implementation because monomorphization across the variants wasn't supported.

### Summum??

It'a just a dumb pun.  I needed a unique name for crates.io.  It means "highest" in Latin.  No connection whatsoever to the [pyramid people in Utah](https://en.wikipedia.org/wiki/Summum).

## Usage

Defining a sum type is just like any other enum:
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
- `pub fn try_borrow_*t*(&self) -> Option<&T>`
- `pub fn borrow_*t*(&self) -> &T`
- `pub fn try_borrow_mut_*t*(&mut self) -> Option<&mut T>`
- `pub fn borrow_mut_*t*(&mut self) -> &mut T`
- `pub fn try_into_*t*(self) -> Option<T>`
- `pub fn into_*t*(self) -> T`

**Note**: `*t*` is a lower_snake_case rendering of the variant identifier or type

¹If you want more accessors (or features in general), please email me  
²Except where the variant type would be an "uncovered" generic as described [here](https://doc.rust-lang.org/error_codes/E0210.html)  

#### Future Plan for Accessors

I'd like to implement generic accessors, along the lines of: `pub fn try_into_sub<T>(self) -> Option<T>`, for example.  This would eliminate the annoyance of remembering/ guessing what identifier is assigned to a particular variant.  Unfortunately that seems to be blcoked on [this issue](https://github.com/rust-lang/rust/issues/20041) for the time being.


GOAT, Todo
*static type name output
*see if I can break the impl up again, and go back to an attrib macro for generic impls

Conversion between union types that share overlapping sub-types
Generic method implementation across all sub-types
Trait-style abstract interface


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

## Acknowledgement & Other Options

Several other union type / sum type crates exist, and they might be better for your use case

- [typeunion by Antonius Naumann](https://github.com/antoniusnaumann/typeunion)
- [sum_type by Michael F. Bryan](https://github.com/Michael-F-Bryan/sum_type)
- [typesum by Natasha England-Elbro](https://github.com/0x00002a/typesum)

Each of them have things they do uniquely well and I took inspiration from all of them.  In particular I used `typeunion` as the basis for this crate.
