#[macro_export]
/// Ruby-like way to crate a hashmap.
macro_rules! hash {
    ( $( $k:expr => $v:expr ),* ) => {
        {
            let mut hash = std::collections::HashMap::new();
            $( hash.insert($k, $v); )*
            hash
        }
    };
}
