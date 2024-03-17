

use summum_types::summum;


summum!{
    enum SumType {
        String(String),
        Int(i64),
    }
}

summum!{
    type HaslellSumType = String | i64;
}

summum!{
    #[derive(Debug, Clone)]
    enum VecOrV<V> {
        Vec(Vec<V>),
        V(V),
    }
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

summum!{
    #[derive(Debug, Clone)]
    type HaskellNestedOrNotRef<'a, V> = &'a Vec<V> as Vec | 
                                        &'a V as V;
}






use std::collections::HashMap;
use std::sync::Arc;

use expr_map::Expr;

/// The ExprMap object, stores expressions that can be used as keys to retrieve values
#[derive(Clone)]
pub struct ExprMap<V> {
    apps: Option<Arc<Self>>,
    vars: NestedOrNotMap<V>,
}

summum!{
    /// Private type to help untangle an ExprMap<ExprMap<V>> with Rust's static type system
    #[derive(Clone)]
    enum NestedOrNotMap<V> {
        Nested(HashMap<usize, ExprMap<V>>),
        Not(HashMap<usize, V>),
    }

    impl<V> NestedOrNotMap<V> {
        pub fn get(&self, idx: &usize) -> Option<NestedOrNotRef<V>> {
            self.get(idx).map(|r| r.into())
        }
        pub fn get_mut(&mut self, idx: &usize) -> Option<NestedOrNotRefMut<V>> {
            self.get_mut(idx).map(|r| r.into())
        }
        pub fn insert(&mut self, idx: usize, val: NestedOrNot<V>) {
            self.insert(idx, val.into_inner_var());
        }
    }

    #[derive(Clone)]
    type NestedOrNot<V> = ExprMap<V> as Nested | V as Not;

    #[derive(Clone)]
    type NestedOrNotRef<'a, V> = &'a ExprMap<V> as Nested | &'a V as Not;

    type NestedOrNotRefMut<'a, V> = &'a mut ExprMap<V> as Nested | &'a mut V as Not;

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
        fn multiply_add_one_inner_var(&self, multiplier: InnerT) -> InnerT {
            *self * multiplier + 1 as InnerT
        }

        // fn max_inner_var() -> InnerT {
        //     Self::MAX
        // }
        fn max_inner_var() -> Self {
            Self::MAX.into()
        }
    }

    impl NumVec {
        fn push(&mut self, item: Num) {
            // This will be expanded into either `into_f64` or `into_i64` depending
            // on the variant branch being generated
            let val = item.into_inner_var();
            self.push(val);
        }
        fn get_inner_var(&self, idx: usize) -> Option<Num> {
            self.get(idx).map(|r| (*r).into())
        }
    }
}




fn main() {

    println!("MAX {:?}", Num::max_f64());

    let mut goat_vec: NumVec = vec![3.0].into();
    // goat_vec.get_i64(0);

    //goat_vec.push(Num::from(2.0));

    println!("GOAT {}", Num::from(1.0).multiply_add_one_f64(3.0));

    // println!("{:?}", SliceOrPie::<usize>::variants());

    // let sliceor: SliceOrPie<_> = vec![40].into();
    // println!("{:?}", sliceor.get(0));
}
