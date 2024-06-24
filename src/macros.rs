#[allow(unused_imports)]
use crate::*;

/** A macro to easily create a [`Language`].

`define_language` derives `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`,
`Hash`, and `Clone` on the given `enum` so it can implement [`Language`].
The macro also implements [`Display`] and [`FromOp`] for the `enum`
based on either the data of variants or the provided strings.

The final variant **must have a trailing comma**; this is due to limitations in
macro parsing.

The language discriminant will use the cases of the enum (the enum discriminant).

See [`LanguageChildren`] for acceptable types of children `Id`s.

Note that you can always implement [`Language`] yourself by just not using this
macro.

Presently, the macro does not support data variant with children, but that may
be added later.

# Example

The following macro invocation shows the the accepted forms of variants:
```
# use egg::*;
define_language! {
    enum SimpleLanguage {
        // string variant with no children
        "pi" = Pi,

        // string variants with an array of child `Id`s (any static size)
        // any type that implements LanguageChildren may be used here
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),

        // can also do a variable number of children in a boxed slice
        // this will only match if the lengths are the same
        "list" = List(Box<[Id]>),

        // string variants with a single child `Id`
        // note that this is distinct from `Sub`, even though it has the same
        // string, because it has a different number of children
        "-"  = Neg(Id),

        // data variants with a single field
        // this field must implement `FromStr` and `Display`
        Num(i32),
        // language items are parsed in order, and we want symbol to
        // be a fallback, so we put it last
        Symbol(Symbol),
        // This is the ultimate fallback, it will parse any operator (as a string)
        // and any number of children.
        // Note that if there were 0 children, the previous branch would have succeeded
        Other(Symbol, Vec<Id>),
    }
}
```
It is also possible to define languages that are generic over some bounded type.

# Example
```rust
# use egg::*;
# use std::{str::FromStr, fmt::{Debug, Display}, hash::Hash};
define_language! {
    enum GenericLang<T: SaturationNumber> {
        Number(T),
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "/" = Div([Id; 2]),
        "*" = Mult([Id; 2]),
     }
}

pub trait SaturationNumber:
    std::fmt::Debug
    + Clone
    + PartialEq
    + FromStr<Err=String> // More specifcally Err should implement `Display`.
    + Ord
    + PartialOrd
    + Hash
    + Display
{
}
```

[`Display`]: std::fmt::Display
[`Debug`]: std::fmt::Debug
[`FromStr`]: std::str::FromStr
[`Hash`]: std::hash::Hash
**/
#[macro_export]
macro_rules! define_language {
    ($(#[$meta:meta])* $vis:vis enum $name:ident $(<$($gen:ident $(: $bound:tt $(+ $bounds:tt)*)?),+>)? { $($variants:tt)* }) => {
        $crate::__define_language!($(#[$meta])* $vis enum $name $(<$($gen $(: $bound $(+ $bounds)*)?),+>)? { $($variants)* } -> {} {} {} {} {} {});
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __define_language {
    // Rule for the end of the enum definition
    ($(#[$meta:meta])* $vis:vis enum $name:ident $(<$($gen:ident $(: $bound:tt $(+ $bounds:tt)*)?),+>)? {} ->
     $decl:tt {$($matches:tt)*} $children:tt $children_mut:tt
     $display:tt {$($from_op:tt)*}
    ) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
        $vis enum $name $(<$($gen $(: $bound $(+ $bounds)*)?),+>)? $decl

        impl<$($($gen $(: $bound $(+ $bounds)*)?),+)? > $crate::Language for $name $(<$($gen),+>)? {
            type Discriminant = std::mem::Discriminant<Self>;

            #[inline(always)]
            fn discriminant(&self) -> Self::Discriminant {
                std::mem::discriminant(self)
            }

            #[inline(always)]
            fn matches(&self, other: &Self) -> bool {
                ::std::mem::discriminant(self) == ::std::mem::discriminant(other) &&
                match (self, other) { $($matches)* _ => false }
            }

            fn children(&self) -> &[Id] { match self $children }
            fn children_mut(&mut self) -> &mut [Id] { match self $children_mut }
        }

        impl<$($($gen $(: $bound $(+ $bounds)*)?),+)? > ::std::fmt::Display for $name $(<$($gen),+>)? {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match (self, f) $display
            }
        }

        impl<$($($gen $(: $bound $(+ $bounds)*)?),+)? > $crate::FromOp for $name $(<$($gen),+>)? {
            type Error = $crate::FromOpError;

            fn from_op(op: &str, children: ::std::vec::Vec<$crate::Id>) -> ::std::result::Result<Self, Self::Error> {
                match (op, children) {
                    $($from_op)*
                    (op, children) => Err($crate::FromOpError::new(op, children)),
                }
            }
        }
    };

    // Rule to handle string variants with no children
    ($(#[$meta:meta])* $vis:vis enum $name:ident $(<$($gen:ident $(: $bound:tt $(+ $bounds:tt)*)?),+>)?
     {
         $string:literal = $variant:ident,
         $($variants:tt)*
     } ->
     { $($decl:tt)* } { $($matches:tt)* } { $($children:tt)* } { $($children_mut:tt)* }
     { $($display:tt)* } { $($from_op:tt)* }
    ) => {
        $crate::__define_language!(
            $(#[$meta])* $vis enum $name $(<$($gen $(: $bound $(+ $bounds)*)?),+>)?
            { $($variants)* } ->
            { $($decl)* $variant, }
            { $($matches)* ($name::$variant, $name::$variant) => true, }
            { $($children)* $name::$variant => &[], }
            { $($children_mut)* $name::$variant => &mut [], }
            { $($display)* ($name::$variant, f) => f.write_str($string), }
            { $($from_op)* ($string, children) if children.is_empty() => Ok($name::$variant), }
        );
    };

    // Rule to handle string variants with an array of child Ids
    ($(#[$meta:meta])* $vis:vis enum $name:ident $(<$($gen:ident $(: $bound:tt $(+ $bounds:tt)*)?),+>)?
     {
         $string:literal = $variant:ident ($ids:ty),
         $($variants:tt)*
     } ->
     { $($decl:tt)* } { $($matches:tt)* } { $($children:tt)* } { $($children_mut:tt)* }
     { $($display:tt)* } { $($from_op:tt)* }
    ) => {
        $crate::__define_language!(
            $(#[$meta])* $vis enum $name $(<$($gen $(: $bound $(+ $bounds)*)?),+>)?
            { $($variants)* } ->
            { $($decl)* $variant($ids), }
            { $($matches)* ($name::$variant(l), $name::$variant(r)) => $crate::LanguageChildren::len(l) == $crate::LanguageChildren::len(r), }
            { $($children)* $name::$variant(ids) => $crate::LanguageChildren::as_slice(ids), }
            { $($children_mut)* $name::$variant(ids) => $crate::LanguageChildren::as_mut_slice(ids), }
            { $($display)* ($name::$variant(..), f) => f.write_str($string), }
            { $($from_op)* (op, children) if op == $string && <$ids as $crate::LanguageChildren>::can_be_length(children.len()) => {
                  let children = <$ids as $crate::LanguageChildren>::from_vec(children);
                  Ok($name::$variant(children))
              },
            }
        );
    };

    // Rule to handle data variants with a single field
    ($(#[$meta:meta])* $vis:vis enum $name:ident $(<$($gen:ident $(: $bound:tt $(+ $bounds:tt)*)?),+>)?
     {
         $variant:ident ($data:ty),
         $($variants:tt)*
     } ->
     { $($decl:tt)* } { $($matches:tt)* } { $($children:tt)* } { $($children_mut:tt)* }
     { $($display:tt)* } { $($from_op:tt)* }
    ) => {
        $crate::__define_language!(
            $(#[$meta])* $vis enum $name $(<$($gen $(: $bound $(+ $bounds)*)?),+>)?
            { $($variants)* } ->
            { $($decl)* $variant($data), }
            { $($matches)* ($name::$variant(data1), $name::$variant(data2)) => data1 == data2, }
            { $($children)* $name::$variant(_data) => &[], }
            { $($children_mut)* $name::$variant(_data) => &mut [], }
            { $($display)* ($name::$variant(data), f) => ::std::fmt::Display::fmt(data, f), }
            { $($from_op)* (op, children) if op.parse::<$data>().is_ok() && children.is_empty() => Ok($name::$variant(op.parse().unwrap())), }
        );
    };

    // Rule to handle data variants with a data field and an array of child Ids
    ($(#[$meta:meta])* $vis:vis enum $name:ident $(<$($gen:ident $(: $bound:tt $(+ $bounds:tt)*)?),+>)?
     {
         $variant:ident ($data:ty, $ids:ty),
         $($variants:tt)*
     } ->
     { $($decl:tt)* } { $($matches:tt)* } { $($children:tt)* } { $($children_mut:tt)* }
     { $($display:tt)* } { $($from_op:tt)* }
    ) => {
        $crate::__define_language!(
            $(#[$meta])* $vis enum $name $(<$($gen $(: $bound $(+ $bounds)*)?),+>)?
            { $($variants)* } ->
            { $($decl)* $variant($data, $ids), }
            { $($matches)* ($name::$variant(d1, l), $name::$variant(d2, r)) => d1 == d2 && $crate::LanguageChildren::len(l) == $crate::LanguageChildren::len(r), }
            { $($children)* $name::$variant(_, ids) => $crate::LanguageChildren::as_slice(ids), }
            { $($children_mut)* $name::$variant(_, ids) => $crate::LanguageChildren::as_mut_slice(ids), }
            { $($display)* ($name::$variant(data, _), f) => ::std::fmt::Display::fmt(data, f), }
            { $($from_op)* (op, children) if op.parse::<$data>().is_ok() && <$ids as $crate::LanguageChildren>::can_be_length(children.len()) => {
                  let data = op.parse::<$data>().unwrap();
                  let children = <$ids as $crate::LanguageChildren>::from_vec(children);
                  Ok($name::$variant(data, children))
              },
            }
        );
    };
}

/** A macro to easily make [`Rewrite`]s.

The `rewrite!` macro greatly simplifies creating simple, purely
syntactic rewrites while also allowing more complex ones.

This panics if [`Rewrite::new`](Rewrite::new()) fails.

The simplest form `rewrite!(a; b => c)` creates a [`Rewrite`]
with name `a`, [`Searcher`] `b`, and [`Applier`] `c`.
Note that in the `b` and `c` position, the macro only accepts a single
token tree (see the [macros reference][macro] for more info).
In short, that means you should pass in an identifier, literal, or
something surrounded by parentheses or braces.

If you pass in a literal to the `b` or `c` position, the macro will
try to parse it as a [`Pattern`] which implements both [`Searcher`]
and [`Applier`].

The macro also accepts any number of `if <expr>` forms at the end,
where the given expression should implement [`Condition`].
For each of these, the macro will wrap the given applier in a
[`ConditionalApplier`] with the given condition, with the first condition being
the outermost, and the last condition being the innermost.

# Example
```
# use egg::*;
use std::borrow::Cow;
use std::sync::Arc;
define_language! {
    enum SimpleLanguage {
        Num(i32),
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),
        "/" = Div([Id; 2]),
    }
}

type EGraph = egg::EGraph<SimpleLanguage, ()>;

let mut rules: Vec<Rewrite<SimpleLanguage, ()>> = vec![
    rewrite!("commute-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
    rewrite!("commute-mul"; "(* ?a ?b)" => "(* ?b ?a)"),

    rewrite!("mul-0"; "(* ?a 0)" => "0"),

    rewrite!("silly"; "(* ?a 1)" => { MySillyApplier("foo") }),

    rewrite!("something_conditional";
             "(/ ?a ?b)" => "(* ?a (/ 1 ?b))"
             if is_not_zero("?b")),
];

// rewrite! supports bidirectional rules too
// it returns a Vec of length 2, so you need to concat
rules.extend(vec![
    rewrite!("add-0"; "(+ ?a 0)" <=> "?a"),
    rewrite!("mul-1"; "(* ?a 1)" <=> "?a"),
].concat());

#[derive(Debug)]
struct MySillyApplier(&'static str);
impl Applier<SimpleLanguage, ()> for MySillyApplier {
    fn apply_one(&self, _: &mut EGraph, _: Id, _: &Subst, _: Option<&PatternAst<SimpleLanguage>>, _: Symbol) -> Vec<Id> {
        panic!()
    }
}

// This returns a function that implements Condition
fn is_not_zero(var: &'static str) -> impl Fn(&mut EGraph, Id, &Subst) -> bool {
    let var = var.parse().unwrap();
    let zero = SimpleLanguage::Num(0);
    // note this check is just an example,
    // checking for the absence of 0 is insufficient since 0 could be merged in later
    // see https://github.com/egraphs-good/egg/issues/297
    move |egraph, _, subst| !egraph[subst[var]].nodes.contains(&zero)
}
```

[macro]: https://doc.rust-lang.org/stable/reference/macros-by-example.html#metavariables
**/
#[macro_export]
macro_rules! rewrite {
    (
        $name:expr;
        $lhs:tt => $rhs:tt
        $(if $cond:expr)*
    )  => {{
        let searcher = $crate::__rewrite!(@parse Pattern $lhs);
        let core_applier = $crate::__rewrite!(@parse Pattern $rhs);
        let applier = $crate::__rewrite!(@applier core_applier; $($cond,)*);
        $crate::Rewrite::new($name.to_string(), searcher, applier).unwrap()
    }};
    (
        $name:expr;
        $lhs:tt <=> $rhs:tt
        $(if $cond:expr)*
    )  => {{
        let name = $name;
        let name2 = String::from(name.clone()) + "-rev";
        vec![
            $crate::rewrite!(name;  $lhs => $rhs $(if $cond)*),
            $crate::rewrite!(name2; $rhs => $lhs $(if $cond)*)
        ]
    }};
}

/** A macro to easily make [`Rewrite`]s using [`MultiPattern`]s.

Similar to the [`rewrite!`] macro,
this macro uses the form `multi_rewrite!(name; multipattern => multipattern)`.
String literals will be parsed a [`MultiPattern`]s.

**/
#[macro_export]
macro_rules! multi_rewrite {
    // limited multipattern support
    (
        $name:expr;
        $lhs:tt => $rhs:tt
    )  => {{
        let searcher = $crate::__rewrite!(@parse MultiPattern $lhs);
        let applier = $crate::__rewrite!(@parse MultiPattern $rhs);
        $crate::Rewrite::new($name.to_string(), searcher, applier).unwrap()
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __rewrite {
    (@parse $t:ident $rhs:literal) => {
        $rhs.parse::<$crate::$t<_>>().unwrap()
    };
    (@parse $t:ident $rhs:expr) => { $rhs };
    (@applier $applier:expr;) => { $applier };
    (@applier $applier:expr; $cond:expr, $($conds:expr,)*) => {
        $crate::ConditionalApplier {
            condition: $cond,
            applier: $crate::__rewrite!(@applier $applier; $($conds,)*)
        }
    };
}

#[cfg(test)]
mod tests {

    use crate::*;

    define_language! {
        enum Simple {
            "+" = Add([Id; 2]),
            "-" = Sub([Id; 2]),
            "*" = Mul([Id; 2]),
            "-" = Neg(Id),
            "list" = List(Box<[Id]>),
            "pi" = Pi,
            Int(i32),
            Var(Symbol),
        }
    }

    #[test]
    fn modify_children() {
        let mut add = Simple::Add([0.into(), 0.into()]);
        add.for_each_mut(|id| *id = 1.into());
        assert_eq!(add, Simple::Add([1.into(), 1.into()]));
    }

    #[test]
    fn some_rewrites() {
        let mut rws: Vec<Rewrite<Simple, ()>> = vec![
            // here it should parse the rhs
            rewrite!("rule"; "cons" => "f"),
            // here it should just accept the rhs without trying to parse
            rewrite!("rule"; "f" => { "pat".parse::<Pattern<_>>().unwrap() }),
        ];
        rws.extend(rewrite!("two-way"; "foo" <=> "bar"));
    }

    #[test]
    #[should_panic(expected = "refers to unbound var ?x")]
    fn rewrite_simple_panic() {
        let _: Rewrite<Simple, ()> = rewrite!("bad"; "?a" => "?x");
    }

    #[test]
    #[should_panic(expected = "refers to unbound var ?x")]
    fn rewrite_conditional_panic() {
        let x: Pattern<Simple> = "?x".parse().unwrap();
        let _: Rewrite<Simple, ()> = rewrite!(
            "bad"; "?a" => "?a" if ConditionEqual::new(x.clone(), x)
        );
    }
    // Testing a "generic" language, where the number is not specifically i32, 
    // but anything that implements some set of traits. For ease of reading 
    // these tests set of traits required for language to accept a generic type
    // is collected under `SaturationNumber`. The fact that the testing suite
    // compiles is testing the macro's parsing implementation.

    #[derive(std::fmt::Debug, Clone, PartialEq, Eq, Ord, PartialOrd, std::hash::Hash)]
    pub struct CustomNumber {
        num: i32,
    }

    impl std::str::FromStr for CustomNumber {
        type Err = String;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let num = s.parse::<i32>().map_err(|e| format!("{e:?}"))?;
            Ok(Self { num })
        }
    }

    impl std::fmt::Display for CustomNumber {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.num)
        }
    }

    impl From<i32> for CustomNumber {
        fn from(value: i32) -> Self {
            Self {
                num: value
            }
        }
    }

    pub trait SaturationNumber:
        std::fmt::Debug
        + Clone
        + PartialEq
        + std::str::FromStr<Err=String>
        + Ord
        + PartialOrd
        + std::hash::Hash
        + std::fmt::Display
    {
    }

    impl SaturationNumber for CustomNumber {}

    define_language! {
        pub enum GenericLang<T: SaturationNumber> {
            Number(T),
            "+" = Add([Id; 2]),
            "-" = Sub([Id; 2]),
            "/" = Div([Id; 2]),
            "*" = Mult([Id; 2]),
        }
    }

    #[test]
    fn test_generic_lang_display() {
        assert_eq!(format!("{}", GenericLang::<CustomNumber>::Add([1.into(), 2.into()])), "+");
    }

    #[test]
    fn test_generic_lang_from_op() {
        let add_op = GenericLang::<CustomNumber>::from_op("+", vec![1.into(), 2.into()]).unwrap();
        if let GenericLang::Add(ids) = add_op {
            assert_eq!(ids, [1.into(), 2.into()]);
        } else {
            panic!("Expected GenericLang::Add variant");
        }
    }
}
