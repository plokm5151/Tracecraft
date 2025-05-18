#!/bin/bash
set -e

# 清除舊測資
rm -rf test_advanced_ws
mkdir -p test_advanced_ws/{lib_base,lib_derive,lib_trait,bin_demo}
mkdir -p test_advanced_ws/lib_base/src
mkdir -p test_advanced_ws/lib_derive/src
mkdir -p test_advanced_ws/lib_trait/src
mkdir -p test_advanced_ws/bin_demo/src

# 建立 Cargo workspace
cat > test_advanced_ws/Cargo.toml <<EOF2
[workspace]
members = [
    "lib_base",
    "lib_derive",
    "lib_trait",
    "bin_demo",
]
EOF2

# lib_base - 含有泛型、宏、alias
cat > test_advanced_ws/lib_base/Cargo.toml <<EOF2
[package]
name = "lib_base"
version = "0.1.0"
edition = "2021"
EOF2

cat > test_advanced_ws/lib_base/src/lib.rs <<EOF2
pub fn base_fn<T>(val: T) -> T { val }
pub mod util_mod {
    pub fn util_fn() {}
}
pub use base_fn as alias_base_fn;
pub use util_mod::util_fn as alias_util_fn;

// 測試 macro call
#[macro_export]
macro_rules! gen_fn {
    (\$name:ident) => {
        pub fn \$name() {}
    };
}
gen_fn!(macro_fn);
EOF2

# lib_trait - 含有 trait 與泛型
cat > test_advanced_ws/lib_trait/Cargo.toml <<EOF2
[package]
name = "lib_trait"
version = "0.1.0"
edition = "2021"
EOF2

cat > test_advanced_ws/lib_trait/src/lib.rs <<EOF2
pub trait Op {
    fn apply(&self, x: i32) -> i32;
}
pub struct Add;
impl Op for Add {
    fn apply(&self, x: i32) -> i32 { x + 10 }
}
pub struct Mul;
impl Op for Mul {
    fn apply(&self, x: i32) -> i32 { x * 10 }
}
EOF2

# lib_derive - alias 到其它 crate
cat > test_advanced_ws/lib_derive/Cargo.toml <<EOF2
[package]
name = "lib_derive"
version = "0.1.0"
edition = "2021"
EOF2

cat > test_advanced_ws/lib_derive/src/lib.rs <<EOF2
pub use lib_base::alias_base_fn as super_base_fn;
pub use lib_base::alias_util_fn as super_util_fn;
pub use lib_trait::Op as SuperOp;
EOF2

# bin_demo - 呼叫全部 alias/trait/generic/macro
cat > test_advanced_ws/bin_demo/Cargo.toml <<EOF2
[package]
name = "bin_demo"
version = "0.1.0"
edition = "2021"
EOF2

cat > test_advanced_ws/bin_demo/src/main.rs <<EOF2
use lib_derive::{super_base_fn, super_util_fn, SuperOp};
use lib_trait::{Add, Mul};

fn run_trait(op: &dyn SuperOp, x: i32) -> i32 {
    op.apply(x)
}
fn main() {
    let add = Add;
    let mul = Mul;
    println!("{}", run_trait(&add, 1));
    println!("{}", run_trait(&mul, 2));
    super_base_fn(123);
    super_util_fn();
}
EOF2

echo "Advanced Rust workspace generated in test_advanced_ws/"
