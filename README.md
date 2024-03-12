# summum-types
A sum-type macro crate with all the features

## Summary

This crate strives to provide dynamic runtime-resolving types on top of Rust’s static compile-time types.

To that end, the `summum` macro allows for easy declaration of sum-types that:
- come with all the traits and methods you'd expect for conversion and access
- allow blanket method implementation across all sub-types
- support abstract interfaces to delegate to sub-type methods

## Discussion

### Summum??

It'a just a dumb pun.  I needed a unique name for crates.io.  It means "highest" in Latin.  No connection whatsoever to the [pyramid people in Utah](https://en.wikipedia.org/wiki/Summum).

### Motivation

I realized I needed something like this when I tried to implement a recursive type definition.  [Rust's static type system could not represent the type](https://users.rust-lang.org/t/recursive-generic-type-parameters-full-featured-union-types/108114) I needed without imposing a finite recursion depth.

## Features

> Rust's `enum`s are already sum types, so why do I need this crate?

Lots and lots of boilerplate written for you.




All the accessors and conveniences you'd expect
Conversion between union types that share overlapping sub-types
Generic method implementation across all sub-types
Trait-style abstract interface



## Acknowledgement & Other Options

Several other union type / sum type crates exist, and they might be better for your use case

- [typeunion by Antonius Naumann](https://github.com/antoniusnaumann/typeunion)
- [sum_type by Michael F. Bryan](https://github.com/Michael-F-Bryan/sum_type)
- [typesum by Natasha England-Elbro](https://github.com/0x00002a/typesum)

Each of them have things they do uniquely well and I took inspiration from all of them.  In particular I used `typeunion` as the basis for this crate.
