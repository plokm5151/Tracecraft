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
