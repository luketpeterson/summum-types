

use summum_types::summum;

summum!{
    enum SumType {
        String(String),
        Int(i64),
    }
}

#[test]
fn simple_sum_type() {
    let _sum: SumType = 42.into();
}

summum!{
    type HaslellSumType = String | i64;
}

#[test]
fn haskell_style_simple() {
    let _sum: HaslellSumType = "Hello".to_string().into();
}

summum!{
    #[derive(Debug, Clone)]
    enum VecOrV<V> where V: Default {
        Vec(Vec<V>),
        V(V),
    }

    impl<V> VecOrV<V> where V: Default {
        fn default_inner_var() -> Self {
            InnerT::default().into()
        }
    }
}

#[test]
fn enum_with_generics_and_derives() {
    let _sum: VecOrV<i64> = vec![42].into();
}

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

#[test]
fn enum_with_lifetime_and_impl() {
    let sop: SliceOrPie<i64> = vec![42].into();
    assert_eq!(sop.get(0), Some(&42));
    assert_eq!(sop.get(3), None);
}

summum!{
    #[derive(Debug, Clone)]
    type HaskellNestedOrNotRef<'a, V> = &'a Vec<V> as Vec | 
                                        &'a V as V;
}

#[test]
fn haskell_with_generics_and_as() {
    let vec = vec![42];
    let vec_ref: HaskellNestedOrNotRef<i64> = (&vec).into();
    assert_eq!(vec_ref.variant_name(), "Vec");
}

summum!{
    #[derive(Debug)]
    enum Num {
        F64(f64),
        I64(i64),
    }

    enum NumVec {
        F64(Vec<f64>),
        I64(Vec<i64>),
    }

    impl Num {
        #[allow(dead_code)]
        fn max_inner_var() -> Self {
            InnerT::MAX.into()
        }
        #[allow(dead_code)]
        fn multiply_add_one_inner_var(&self, multiplier: InnerT) -> InnerT {
            *self * multiplier + 1 as InnerT
        }
    }

    impl NumVec {
        fn push(&mut self, item: Num) {
            // This will be expanded into either `into_f64` or `into_i64` depending
            // on the variant branch being generated
            let val = item.into_inner_var();
            self.push(val);
        }
        #[allow(dead_code)]
        fn get_inner_var(&self, idx: usize) -> Option<Num> {
            self.get(idx).map(|r| (*r).into())
        }
    }
}

#[test]
fn cross_type_interop() {
    assert_eq!(Num::max_i64().into_i64(), i64::MAX);
    assert_eq!(Num::from(1.0).multiply_add_one_f64(3.0), 4.0);

    let mut vec: NumVec = Vec::<i64>::new().into();
    vec.push(42.into());
    assert_eq!(vec.get_i64(0).unwrap().into_i64(), 42);
}

summum!{
    #[derive(Debug, PartialEq)]
    enum NumAgain {
        F64(f64),
        I64(i64),
    }

    impl NumAgain {
        fn multiply_int_only(&self, other: i64) -> Self {
            summum_restrict!(I64);
            (*self * other).into()
        }
        fn convert_to_float_without_rounding(&self) -> f64 {
            if *self > i32::MAX as InnerT {
                summum_exclude!(I64, ); //You can supply multiple variants
                *self as f64
            } else {
                *self as f64
            }
        }
    }
}

#[test]
fn restrict_and_exclude() {
    assert_eq!(NumAgain::from(2).multiply_int_only(2), 4.into());
    assert_eq!(NumAgain::from(120000000).convert_to_float_without_rounding(), 120000000.0);
}

#[test]
#[should_panic]
fn restrict_and_exclude_panic1() {
    assert_eq!(NumAgain::from(2.0).multiply_int_only(2), 4.into());
}

#[test]
#[should_panic]
fn restrict_and_exclude_panic2() {
    assert_eq!(NumAgain::from(12000000000).convert_to_float_without_rounding(), 12000000000.0);
}


summum!{
    #[allow(dead_code)]
    #[derive(Clone)]
    struct EM<V> variants<T> {
        #[derive(Default)]
        Nested(T=Self),
        #[derive(Default)]
        Not(T=V),
    } {
        apps: Option<Box<Self>>,
        inners: Vec<InnerT>,
        vars: Vec<T>,
    }

    impl<V> EM<V> where V: Default {
        /// Testing invoking the default implemented by the default macro
        fn default_a_inner_var() -> Self {
            InnerT::default().into()
        }
        /// Testing invoking the other default
        fn default_b_inner_var() -> Self {
            Self::default_a_inner_var().into_inner_var()
        }
    }
}

#[test]
fn validate_sub_type_structures() {
    //NOTE: this test validates the sub-structs are created with the correct types, however
    // this is ABSOLUTELY NOT the recommended way to use this feature.  This feature is
    // intended to keep the sub_type structs abstracted away as much as possible
    let new_sub_struct = EMNot::<usize> {
        apps: None,
        inners: vec![],
        vars: vec![42 as usize],
    };
    let new_parent_enum: EM<usize> = new_sub_struct.clone().into();
    let _other_sub_struct = EMNot::<usize> {
        apps: Some(Box::new(new_parent_enum)),
        inners: vec![new_sub_struct],
        vars: vec![],
    };
}
















// use std::collections::HashMap;
// use std::sync::Arc;

// use expr_map::Expr;

// /// The ExprMap object, stores expressions that can be used as keys to retrieve values
// #[derive(Clone)]
// pub struct ExprMap<V> {
//     apps: Option<Arc<Self>>,
//     vars: NestedOrNotMap<V>,
// }

// summum!{
//     /// Private type to help untangle an ExprMap<ExprMap<V>> with Rust's static type system
//     #[derive(Clone)]
//     enum NestedOrNotMap<V> {
//         Nested(HashMap<usize, ExprMap<V>>),
//         Not(HashMap<usize, V>),
//     }

//     impl<V> NestedOrNotMap<V> {
//         pub fn get(&self, idx: &usize) -> Option<NestedOrNotRef<V>> {
//             self.get(idx).map(|r| r.into())
//         }
//         pub fn get_mut(&mut self, idx: &usize) -> Option<NestedOrNotRefMut<V>> {
//             self.get_mut(idx).map(|r| r.into())
//         }
//         pub fn insert(&mut self, idx: usize, val: NestedOrNot<V>) {
//             self.insert(idx, val.into_inner_var());
//         }
//     }

//     #[derive(Clone)]
//     type NestedOrNot<V> = ExprMap<V> as Nested | V as Not;

//     #[derive(Clone)]
//     type NestedOrNotRef<'a, V> = &'a ExprMap<V> as Nested | &'a V as Not;

//     type NestedOrNotRefMut<'a, V> = &'a mut ExprMap<V> as Nested | &'a mut V as Not;

// }
