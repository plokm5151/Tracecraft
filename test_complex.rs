mod submod {
    pub struct Data;
    impl Data {
        pub fn process(&self) {}
    }
}
use submod::Data as D;

fn util<T>(x: T) {}
fn main() {
    let d = D;
    d.process();
    util::<i32>(42);
    helper();
}
fn helper() {
    println!("helping");
}
