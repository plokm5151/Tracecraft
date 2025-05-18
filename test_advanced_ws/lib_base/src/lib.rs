pub fn base_fn<T>(val: T) -> T { val }
pub mod util_mod {
    pub fn util_fn() {}
}
pub use base_fn as alias_base_fn;
pub use util_mod::util_fn as alias_util_fn;

// 測試 macro call
#[macro_export]
macro_rules! gen_fn {
    ($name:ident) => {
        pub fn $name() {}
    };
}
gen_fn!(macro_fn);
