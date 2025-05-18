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
