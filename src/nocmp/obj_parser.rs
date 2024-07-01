use nalgebra::Vector3;

#[derive(Debug)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub normal: Option<Vector3<f32>>,
    pub tex_coord: Option<(f32, f32)>,
}

#[derive(Debug)]
pub struct Face {
    pub vertices: Vec<usize>, // indices of the vertices
    pub normal_indices: Vec<usize>,
    pub tex_coord_indices: Vec<usize>,
    pub smoothing_group: Option<u32>,
}

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
}

impl Mesh {
    pub fn new() -> Self {
        Mesh {
            vertices: Vec::new(),
            faces: Vec::new(),
        }
    }
}

use std::fs::File;
use std::io::{BufRead, BufReader};

impl Mesh {
    pub fn parse_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {

        //A good start, but it does not actually do anything with the smoothing group
        //And a Face here contains indices into each data array, i.e,
        //Into Position, into Normals, into texture coords
        //But our realtime mesh will only have ONE index
        //into a vertex format that has all needed things associated with it.


        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut tex_coords = Vec::new();
        let mut faces = Vec::new();
        let mut current_smoothing_group = None;

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    // Vertex position
                    let x: f32 = parts[1].parse()?;
                    let y: f32 = parts[2].parse()?;
                    let z: f32 = parts[3].parse()?;
                    vertices.push(Vector3::new(x, y, z));
                }
                "vn" => {
                    // Vertex normal
                    let x: f32 = parts[1].parse()?;
                    let y: f32 = parts[2].parse()?;
                    let z: f32 = parts[3].parse()?;
                    normals.push(Vector3::new(x, y, z));
                }
                "vt" => {
                    // Vertex texture coordinate
                    let u: f32 = parts[1].parse()?;
                    let v: f32 = parts[2].parse()?;
                    tex_coords.push((u, v));
                }
                "f" => {
                    // Face
                    let mut vertex_indices = Vec::new();
                    let mut normal_indices = Vec::new();
                    let mut tex_coord_indices = Vec::new();

                    for part in &parts[1..] {
                        let indices: Vec<&str> = part.split('/').collect();

                        let vertex_index: usize = indices[0].parse()?;
                        vertex_indices.push(vertex_index - 1);

                        if indices.len() > 1 && !indices[1].is_empty() {
                            let tex_coord_index: usize = indices[1].parse()?;
                            tex_coord_indices.push(tex_coord_index - 1);
                        }

                        if indices.len() > 2 && !indices[2].is_empty() {
                            let normal_index: usize = indices[2].parse()?;
                            normal_indices.push(normal_index - 1);
                        }
                    }

                    faces.push(Face {
                        vertices: vertex_indices,
                        normal_indices,
                        tex_coord_indices,
                        smoothing_group: current_smoothing_group,
                    });
                }
                "s" => {
                    // Smoothing group
                    if parts[1] == "off" {
                        current_smoothing_group = None;
                    } else {
                        current_smoothing_group = Some(parts[1].parse()?);
                    }
                }
                _ => {}
            }
        }

        let vertices = vertices
            .into_iter()
            .enumerate()
            .map(|(i, position)| Vertex {
                position,
                normal: normals.get(i).cloned(),
                tex_coord: tex_coords.get(i).cloned(),
            })
            .collect();

        Ok(Mesh { vertices, faces })
    }
}