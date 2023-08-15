use bvparty::run;
use bvparty::nocmp::spline_curves;

fn main() {

    let bezP0 = spline_curves::CurvePoint {x:0.0,y:0.0,z:0.0};
    let bezP1 = spline_curves::CurvePoint {x:0.0,y:1.0,z:0.0};
    let bezP2 = spline_curves::CurvePoint {x:1.0,y:1.0,z:0.0};
    let bezP3 = spline_curves::CurvePoint {x:1.0,y:0.0,z:0.0};

    let bezzyPs = [bezP0,bezP1,bezP2,bezP3];
    let bezCalc = spline_curves::do_bezzy(&bezzyPs, 0.5);
    println!("Hello, Bezier {} , {} , {}!",bezCalc.x,bezCalc.y,bezCalc.z);
    println!("Hello, world!");
    pollster::block_on(run());
}