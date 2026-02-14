// =============================================================================
// Recursive accumulator macros
// =============================================================================
//
// These macros generate all tuple arities from a flat list of elements.
// Each element is listed once; the macro automatically generates all
// prefix arities (1-element, 2-element, ..., N-element tuples).

/// Recursive accumulator for type-only callbacks.
macro_rules! seq_types {
    (@acc $callback:ident [$($acc:ident),*]) => {};
    (@acc $callback:ident [$($acc:ident),*] $next:ident $($rest:ident)*) => {
        $callback!($($acc,)* $next);
        seq_types!(@acc $callback [$($acc,)* $next] $($rest)*);
    };
}

/// Recursive accumulator for dual-type callbacks.
macro_rules! seq_dual {
    (@acc $callback:ident [$($sa:ident),*] [$($da:ident),*]) => {};
    (@acc $callback:ident [$($sa:ident),*] [$($da:ident),*] ($s:ident, $d:ident) $($rest:tt)*) => {
        $callback!($($sa,)* $s; $($da,)* $d);
        seq_dual!(@acc $callback [$($sa,)* $s] [$($da,)* $d] $($rest)*);
    };
}

/// Recursive accumulator for type+index callbacks.
macro_rules! seq_tuples {
    (@acc $callback:ident [$($aT:ident),*] [$($ai:tt),*]) => {};
    (@acc $callback:ident [$($aT:ident),*] [$($ai:tt),*] ($T:ident, $i:tt) $($rest:tt)*) => {
        $callback!($($aT,)* $T; $($ai,)* $i);
        seq_tuples!(@acc $callback [$($aT,)* $T] [$($ai,)* $i] $($rest)*);
    };
    ($callback:ident; $($pairs:tt)+) => {
        seq_tuples!(@acc $callback [] [] $($pairs)+);
    };
    (@from $callback:ident [$($aT:ident),*] [$($ai:tt),*]; $($pairs:tt)+) => {
        seq_tuples!(@acc $callback [$($aT),*] [$($ai),*] $($pairs)+);
    };
}

// =============================================================================
// Constraint tuple macros (used by constraint.rs)
// =============================================================================

/// Calls `$callback!(T0)`, ..., `$callback!(T0, ..., T15)` for arities 1..16.
macro_rules! with_tuple_sizes {
    ($callback:ident) => {
        seq_types!(@acc $callback [] T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 T12 T13 T14 T15);
    };
}

/// Calls `$callback!(S0; D0)`, ..., `$callback!(S0..S15; D0..D15)` for arities 1..16.
macro_rules! with_dual_tuple_sizes {
    ($callback:ident) => {
        seq_dual!(@acc $callback [] [] (S0,D0) (S1,D1) (S2,D2) (S3,D3) (S4,D4) (S5,D5) (S6,D6) (S7,D7) (S8,D8) (S9,D9) (S10,D10) (S11,D11) (S12,D12) (S13,D13) (S14,D14) (S15,D15));
    };
}

// =============================================================================
// ToSQL tuple impls
// =============================================================================

use crate::{SQL, SQLParam, ToSQL, Token};

/// Callback: implements `ToSQL` for a tuple of the given arity.
macro_rules! impl_tosql_tuple {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<'a, V: SQLParam, $($T: ToSQL<'a, V>),+> ToSQL<'a, V> for ($($T,)+) {
            fn to_sql(&self) -> SQL<'a, V> {
                SQL::join([$(self.$idx.to_sql(),)+], Token::COMMA)
            }
        }
    };
}

/// Calls `$callback!` for arities 1..=8.
macro_rules! with_col_sizes_8 {
    ($callback:ident) => {
        seq_tuples!($callback;
            (T0,0) (T1,1) (T2,2) (T3,3)
            (T4,4) (T5,5) (T6,6) (T7,7)
        );
    };
}

/// Calls `$callback!` for arities 9..=16.
#[allow(unused_macros)]
macro_rules! with_col_sizes_16 {
    ($callback:ident) => {
        seq_tuples!(@from $callback
            [T0,T1,T2,T3,T4,T5,T6,T7]
            [0,1,2,3,4,5,6,7];
            (T8,8) (T9,9) (T10,10) (T11,11)
            (T12,12) (T13,13) (T14,14) (T15,15)
        );
    };
}

/// Calls `$callback!` for arities 17..=32.
#[allow(unused_macros)]
macro_rules! with_col_sizes_32 {
    ($callback:ident) => {
        seq_tuples!(@from $callback
            [T0,T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15]
            [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15];
            (T16,16) (T17,17) (T18,18) (T19,19)
            (T20,20) (T21,21) (T22,22) (T23,23)
            (T24,24) (T25,25) (T26,26) (T27,27)
            (T28,28) (T29,29) (T30,30) (T31,31)
        );
    };
}

/// Calls `$callback!` for arities 33..=64.
#[allow(unused_macros)]
macro_rules! with_col_sizes_64 {
    ($callback:ident) => {
        seq_tuples!(@from $callback
            [T0,T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,
             T16,T17,T18,T19,T20,T21,T22,T23,T24,T25,T26,T27,T28,T29,T30,T31]
            [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,
             16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31];
            (T32,32) (T33,33) (T34,34) (T35,35)
            (T36,36) (T37,37) (T38,38) (T39,39)
            (T40,40) (T41,41) (T42,42) (T43,43)
            (T44,44) (T45,45) (T46,46) (T47,47)
            (T48,48) (T49,49) (T50,50) (T51,51)
            (T52,52) (T53,53) (T54,54) (T55,55)
            (T56,56) (T57,57) (T58,58) (T59,59)
            (T60,60) (T61,61) (T62,62) (T63,63)
        );
    };
}

/// Calls `$callback!` for arities 65..=128.
#[allow(unused_macros)]
macro_rules! with_col_sizes_128 {
    ($callback:ident) => {
        seq_tuples!(@from $callback
            [T0,T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,
             T16,T17,T18,T19,T20,T21,T22,T23,T24,T25,T26,T27,T28,T29,T30,T31,
             T32,T33,T34,T35,T36,T37,T38,T39,T40,T41,T42,T43,T44,T45,T46,T47,
             T48,T49,T50,T51,T52,T53,T54,T55,T56,T57,T58,T59,T60,T61,T62,T63]
            [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,
             16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,
             32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,
             48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63];
            (T64,64) (T65,65) (T66,66) (T67,67)
            (T68,68) (T69,69) (T70,70) (T71,71)
            (T72,72) (T73,73) (T74,74) (T75,75)
            (T76,76) (T77,77) (T78,78) (T79,79)
            (T80,80) (T81,81) (T82,82) (T83,83)
            (T84,84) (T85,85) (T86,86) (T87,87)
            (T88,88) (T89,89) (T90,90) (T91,91)
            (T92,92) (T93,93) (T94,94) (T95,95)
            (T96,96) (T97,97) (T98,98) (T99,99)
            (T100,100) (T101,101) (T102,102) (T103,103)
            (T104,104) (T105,105) (T106,106) (T107,107)
            (T108,108) (T109,109) (T110,110) (T111,111)
            (T112,112) (T113,113) (T114,114) (T115,115)
            (T116,116) (T117,117) (T118,118) (T119,119)
            (T120,120) (T121,121) (T122,122) (T123,123)
            (T124,124) (T125,125) (T126,126) (T127,127)
        );
    };
}

/// Calls `$callback!` for arities 129..=200.
#[allow(unused_macros)]
macro_rules! with_col_sizes_200 {
    ($callback:ident) => {
        seq_tuples!(@from $callback
            [T0,T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,
             T16,T17,T18,T19,T20,T21,T22,T23,T24,T25,T26,T27,T28,T29,T30,T31,
             T32,T33,T34,T35,T36,T37,T38,T39,T40,T41,T42,T43,T44,T45,T46,T47,
             T48,T49,T50,T51,T52,T53,T54,T55,T56,T57,T58,T59,T60,T61,T62,T63,
             T64,T65,T66,T67,T68,T69,T70,T71,T72,T73,T74,T75,T76,T77,T78,T79,
             T80,T81,T82,T83,T84,T85,T86,T87,T88,T89,T90,T91,T92,T93,T94,T95,
             T96,T97,T98,T99,T100,T101,T102,T103,T104,T105,T106,T107,T108,T109,T110,T111,
             T112,T113,T114,T115,T116,T117,T118,T119,T120,T121,T122,T123,T124,T125,T126,T127]
            [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,
             16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,
             32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,
             48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,
             64,65,66,67,68,69,70,71,72,73,74,75,76,77,78,79,
             80,81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,
             96,97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,
             112,113,114,115,116,117,118,119,120,121,122,123,124,125,126,127];
            (T128,128) (T129,129) (T130,130) (T131,131)
            (T132,132) (T133,133) (T134,134) (T135,135)
            (T136,136) (T137,137) (T138,138) (T139,139)
            (T140,140) (T141,141) (T142,142) (T143,143)
            (T144,144) (T145,145) (T146,146) (T147,147)
            (T148,148) (T149,149) (T150,150) (T151,151)
            (T152,152) (T153,153) (T154,154) (T155,155)
            (T156,156) (T157,157) (T158,158) (T159,159)
            (T160,160) (T161,161) (T162,162) (T163,163)
            (T164,164) (T165,165) (T166,166) (T167,167)
            (T168,168) (T169,169) (T170,170) (T171,171)
            (T172,172) (T173,173) (T174,174) (T175,175)
            (T176,176) (T177,177) (T178,178) (T179,179)
            (T180,180) (T181,181) (T182,182) (T183,183)
            (T184,184) (T185,185) (T186,186) (T187,187)
            (T188,188) (T189,189) (T190,190) (T191,191)
            (T192,192) (T193,193) (T194,194) (T195,195)
            (T196,196) (T197,197) (T198,198) (T199,199)
        );
    };
}

// =============================================================================
// Feature-gated invocations
// =============================================================================

with_col_sizes_8!(impl_tosql_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_tosql_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_tosql_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_tosql_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_tosql_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_tosql_tuple);
