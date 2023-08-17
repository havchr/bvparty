use matrix_operations::matrix;
use anyhow::*;

/*
This is spline/curve code based on the really nice video from Freya Holmér found here :
https://www.youtube.com/watch?v=jvPPXbo87ds&ab_channel=FreyaHolm%C3%A9r
It implements different curves by using different coeffecion matrices for the cubic function.
 */

//todo - serde to save and load curve data. ?

//todo - easing functions for the 0-1 case ease-out , ease-in etc...

//todo - a good way to read curve data , a good way to get curve data (export from blender?)
//todo curve editor?
//todo - we should have a proper (math)Vector struct that can do all math things and such
//todo - we should be able to set constrains on connected points in a bezier spline, tangent can
// be 1) mirrored, 2) mirrored but scaled 3) free
//Mirrored , typically we will need a vector from point and then just *-1 that vector I guess?

use serde::{Deserialize, Serialize};

#[derive(Clone,Serialize,Deserialize)]
pub struct CurvePoint {
   pub x: f32,
    pub y: f32,
    pub z: f32
}
pub fn do_bezzy_spline_t_01(points: &[CurvePoint], t : f32) -> anyhow::Result<CurvePoint> {
    //We want a spline with 5 points to go to 0-2 because we have 0,1,2,3 and then 1,2,3,4
    if points.len() % 4 != 0 {
        return Err(anyhow::anyhow!("spline points not multiple of 4"));
    }
    let t_whole_number = points.len() as f32 / 4.0;
    Ok(do_bezzy_spline(&points,t*t_whole_number))
}

pub fn do_bezzy_super_spline_t_01(points: &[CurvePoint], t : f32) -> CurvePoint {
    //We want a spline with 5 points to go to 0-2 because we have 0,1,2,3 and then 1,2,3,4
    let t_whole_number = points.len() as f32 / 4.0;
    do_bezzy_spline_duplicate_end_points(&points,t*t_whole_number)
}

pub fn do_bezzy_spline_duplicate_end_points(points: &[CurvePoint], t : f32) -> CurvePoint {
    //assuming n num of points,
    //for points 0-3 , we are using original points, but then 3,4,5,6 , 6,7,8,9
    let index_start= (t.floor() as i32*4 -1).max(0) as usize;
    let t_local= t.fract();

    do_bezzy(
        &points[index_start],
        &points[index_start+1],
        &points[index_start+2],
        &points[index_start+3],
        t_local)
}

pub fn do_bezzy_spline(points: &[CurvePoint], t : f32) -> CurvePoint {
   //assuming n num of points,
    //if we are 1.2
    let index_start= t.floor() as usize *4;
    let index_end =  index_start +4;
    let t_local= t.fract();
    if index_end > points.len() {
      return points[points.len()-1].clone();
    }
    do_bezzy(
        &points[index_start],
        &points[index_start+1],
        &points[index_start+2],
        &points[index_start+3],
        t_local)
}
/// Bezier, Use case - shapes, fonts, vector graphics
/// Continuity C^0/C^1 tangents are manual, interpol - some (hits some of its points directly)
pub fn do_bezzy(p0: &CurvePoint, p1: &CurvePoint, p2: &CurvePoint, p3: &CurvePoint , t : f32) -> CurvePoint {
    let coefs = matrix![
            [ 1.0,    0.0,    0.0,    0.0],
            [-3.0,    3.0,    0.0,    0.0],
            [ 3.0,   -6.0,    3.0,    0.0],
            [-1.0,    3.0,   -3.0,    1.0],
            ];

    let px = matrix![
            [ p0.x],
            [ p1.x],
            [ p2.x],
            [ p3.x],
            ];
    let py = matrix![
            [ p0.y],
            [ p1.y],
            [ p2.y],
            [ p3.y],
            ];
    let pz = matrix![
            [ p0.z],
            [ p1.z],
            [ p2.z],
            [ p3.z],
            ];

    let coeffex_px = coefs.clone() * px;
    let coeffex_py = coefs.clone() * py;
    let coeffex_pz = coefs.clone() * pz;

    let t_mat = matrix![
            [ 1.0,t,t*t,t*t*t],
            ];

    let res_x = t_mat.clone() * coeffex_px;
    let res_y = t_mat.clone() * coeffex_py;
    let res_z = t_mat.clone() * coeffex_pz;

    CurvePoint {
        x:res_x[0][0],
        y:res_y[0][0],
        z:res_z[0][0]
    }
}

/// catmull-rom, Use case - animation, & path smoothing
/// Continuity C^1 tangents auto, interpol - all (hits all  of its points directly)
///point_velocity_interleaved array has point_0 then velcoity_0 then point_1, then velocity_1
pub fn do_catmull_rom(points: &[CurvePoint;4], t : f32) -> CurvePoint {
    let coefs = matrix![
            [ 0.0,    2.0,    0.0,    0.0],
            [-1.0,    0.0,    1.0,    0.0],
            [ 2.0,   -5.0,    4.0,   -1.0],
            [-1.0,    3.0,   -3.0,    1.0],
            ];
    let p0 = &points[0];
    let p1 = &points[1];
    let p2 = &points[2];
    let p3 = &points[3];

    let px = matrix![
            [ p0.x],
            [ p1.x],
            [ p2.x],
            [ p3.x],
            ];
    let py = matrix![
            [ p0.y],
            [ p1.y],
            [ p2.y],
            [ p3.y],
            ];
    let pz = matrix![
            [ p0.z],
            [ p1.z],
            [ p2.z],
            [ p3.z],
            ];

    let coeffex_px = coefs.clone() * px;
    let coeffex_py = coefs.clone() * py;
    let coeffex_pz = coefs.clone() * pz;

    let t_mat = matrix![
            [ 1.0,t,t*t,t*t*t],
            ]*0.5;

    let res_x = t_mat.clone() * coeffex_px;
    let res_y = t_mat.clone() * coeffex_py;
    let res_z = t_mat.clone() * coeffex_pz;

    CurvePoint {
        x:res_x[0][0],
        y:res_y[0][0],
        z:res_z[0][0]
    }
}

/// b-spline , Use case - camera path, curvature sensitive shapes
/// Continuity C^2 tangents, auto, interpol - none(hits non of its points directly)
pub fn do_b_spline(points: &[CurvePoint;4], t : f32) -> CurvePoint {
    let coefs = matrix![
            [ 1.0,    4.0,    1.0,    0.0],
            [-3.0,    0.0,    3.0,    0.0],
            [ 3.0,   -6.0,    3.0,    0.0],
            [-1.0,    3.0,   -3.0,    1.0],
            ];
    let p0 = &points[0];
    let p1 = &points[1];
    let p2 = &points[2];
    let p3 = &points[3];

    let px = matrix![
            [ p0.x],
            [ p1.x],
            [ p2.x],
            [ p3.x],
            ];
    let py = matrix![
            [ p0.y],
            [ p1.y],
            [ p2.y],
            [ p3.y],
            ];
    let pz = matrix![
            [ p0.z],
            [ p1.z],
            [ p2.z],
            [ p3.z],
            ];

    let coeffex_px = coefs.clone() * px;
    let coeffex_py = coefs.clone() * py;
    let coeffex_pz = coefs.clone() * pz;

    let t_mat = matrix![
            [ 1.0,t,t*t,t*t*t],
            ]*(1.0/6.0);

    let res_x = t_mat.clone() * coeffex_px;
    let res_y = t_mat.clone() * coeffex_py;
    let res_z = t_mat.clone() * coeffex_pz;

    CurvePoint {
        x:res_x[0][0],
        y:res_y[0][0],
        z:res_z[0][0]
    }
}

/// hermite , Use case - animation, physics sim , interpolation
/// Continuity C^0/C^1 tangents, explicit (velocity), , interpol - all (hits all  of its points directly)
///point_velocity_interleaved array has point_0 then velcoity_0 then point_1, then velocity_1
pub fn do_hermite(point_velocity_interleaved_array: &[CurvePoint;4], t : f32) -> CurvePoint {
    let coefs = matrix![
            [ 1.0,    0.0,    0.0,    0.0],
            [ 0.0,    1.0,    0.0,    0.0],
            [ -3.0,  -2.0,    3.0,    -1.0],
            [ 2.0,    1.0,   -2.0,    1.0],
            ];
    let p0 = &point_velocity_interleaved_array[0];
    let v0 = &point_velocity_interleaved_array[1];
    let p1 = &point_velocity_interleaved_array[2];
    let v1 = &point_velocity_interleaved_array[3];

    let px = matrix![
            [ p0.x],
            [ v0.x],
            [ p1.x],
            [ v1.x],
            ];
    let py = matrix![
            [ p0.y],
            [ v0.y],
            [ p1.y],
            [ v1.y],
            ];
    let pz = matrix![
            [ p0.z],
            [ v0.z],
            [ p1.z],
            [ v1.z],
            ];

    let coeffex_px = coefs.clone() * px;
    let coeffex_py = coefs.clone() * py;
    let coeffex_pz = coefs.clone() * pz;

    let t_mat = matrix![
            [ 1.0,t,t*t,t*t*t],
            ];

    let res_x = t_mat.clone() * coeffex_px;
    let res_y = t_mat.clone() * coeffex_py;
    let res_z = t_mat.clone() * coeffex_pz;

    CurvePoint {
        x:res_x[0][0],
        y:res_y[0][0],
        z:res_z[0][0]
    }
}