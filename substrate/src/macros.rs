#[macro_export]
macro_rules! into_vec {
    () => (
        ::std::vec![]
    );
    ($elem:expr; $n:expr) => (
        ::std::vec![::std::convert::Into::into($elem), $n]
    );
    ($($x:expr),+ $(,)?) => (
        ::std::vec![$(::std::convert::Into::into($x),)+]
    );
}

#[macro_export]
macro_rules! into_grid {
    () => {
        ::grid::grid![]
    };
    ( [$( $x:expr ),* ]) => {
        ::grid::grid![[$( ::std::convert::Into::into($x) ),*]]
    };
    ( $([$( $x:expr ),*])* ) => {
        ::grid::grid![
            $([$( ::std::convert::Into::into($x) ),*])*
        ]
    };
}
