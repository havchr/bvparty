use bvparty::run;

fn main() {
    println!("Hello, world!");
    pollster::block_on(run());
}
