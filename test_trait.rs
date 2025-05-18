trait Op {
    fn apply(&self, x: i32) -> i32;
}
struct Add;
impl Op for Add {
    fn apply(&self, x: i32) -> i32 { x + 1 }
}
struct Mul;
impl Op for Mul {
    fn apply(&self, x: i32) -> i32 { x * 2 }
}

fn run(op: &dyn Op, x: i32) -> i32 {
    op.apply(x)
}
fn main() {
    let add = Add;
    let mul = Mul;
    run(&add, 5);
    run(&mul, 10);
}
