#[macro_export]
macro_rules! tx_chain {
    ( $first:expr $(, $rest:expr )* $(,)? ) => {{
        let tx = $first;
        $(
            let tx = $crate::transaction::TxCombinator { t1: tx, t2: $rest };
        )*
        tx
    }};
}

#[macro_export]
macro_rules! impl_add {
    ( $( ($lhs:ty, $rhs:ty) ),* ) => {
        $(
            impl std::ops::Add<$rhs> for $lhs {
                type Output = $crate::transaction::TxCombinator<$lhs, $rhs>;

                fn add(self, rhs: $rhs) -> Self::Output {
                    $crate::transaction::TxCombinator { t1: self, t2: rhs }
                }
            }
        )*
    };
}

