#![allow(unused)]

use proptest::prelude::*;
use hvm::syntax::Oper;

const MAX_U60: u64 = !0 >> 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Op(Oper);

// specifies the range of different identifiers in an expression
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdetifierMax(usize);

impl Default for IdetifierMax {
    fn default() -> Self {
        // ensures by default there are variabes in expressions by default
        Self(10)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LesserTerm {
  Identifier { name: usize },
  Lam { name: usize, body: Box<LesserTerm> },
  App { func: Box<LesserTerm>, argm: Box<LesserTerm> },
  U60 { numb: u64 },
  F60 { numb: f64 },
  Op2 { oper: Op, val0: Box<LesserTerm>, val1: Box<LesserTerm> },
}

impl Arbitrary for Op {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        use Oper::*;
        let op_arr = [Add, Sub, Mul, Div, Mod, And, Or,  Xor, Shl, Shr, Lte, Ltn, Eql, Gte, Gtn, Neq];
        (0usize..15).prop_map(move |i| Op(op_arr[i])).boxed()
    }
}

impl Arbitrary for LesserTerm {
    type Parameters = IdetifierMax;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(params: Self::Parameters) -> Self::Strategy {
        use LesserTerm::*;
        let ident_strat = (0..(params.0)).prop_map(|name| Identifier{ name });
        let u60_strat = (0..MAX_U60).prop_map(|numb| U60 {numb});
        // remove the last 4 bits to ensure floats match after conversion
        let f60_strat = proptest::num::f64::NORMAL.prop_map(|x| F60 {numb: f64::from_bits((x.to_bits() >> 4) << 4)});
        prop_oneof![ u60_strat, f60_strat, ident_strat].prop_recursive(
            4, // No more than 4 branch levels deep
            64, // Target around 64 total elements
            16, // Each collection is up to 16 elements long
            move |element| {
                let boxed_elem = element.prop_map(|x| Box::new(x));
                prop_oneof![
                    (0..params.0, boxed_elem.clone()).prop_map(|(name, body)| Lam { name, body }),
                    (boxed_elem.clone(), boxed_elem.clone()).prop_map(|(func, argm)| App { func, argm }),
                    (proptest::arbitrary::any::<Op>(), boxed_elem.clone(), boxed_elem.clone()).prop_map(|(oper, val0, val1)| Op2 { oper, val0, val1 }),
                ]
        }).boxed()
    }
}
