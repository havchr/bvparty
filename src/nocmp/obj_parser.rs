use nalgebra::Vector3;
use nalgebra::Vector2;
use std::collections::HashMap;
use std::error::Error;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct BVec3(Vector3<f32>);

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct BVec2(Vector2<f32>);

#[repr(C)]
#[derive(Copy,Clone, Debug,bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: BVec3,
    pub normal: BVec3,
    pub tex_coord: BVec2,
}

unsafe impl Zeroable for BVec3 {}
unsafe impl Pod for BVec3 {}

unsafe impl Zeroable for BVec2 {}
unsafe impl Pod for BVec2 {}

#[repr(C)]
#[derive(Copy,Clone, Debug,bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjLoaderRealtimeVertex {
    position: [f32;3],
    normal: [f32;3],
    uv: [f32;2],
}

#[repr(C)]
#[derive(Copy,Clone, Debug,bytemuck::Pod, bytemuck::Zeroable)]
pub struct Face{
    pub face_indices: [u32;3] // indices of the vertices
}


/*
MultiIndexing Faces contains multiple lists with index information
so, to find the normal for a face a face, we go like
n1  = normals[face.normal_indices[0]]
n2  = normals[face.normal_indices[1]]
n3  = normals[face.normal_indices[2]]

but realtime 3D really likes it if our face has one index like this :
vertex1 = vertices[indices[0]]
vertex2 = vertices[indices[1]]
vertex3 = vertices[indices[2]]

position1 = vertex1.position
normal1= vertex1.normal
and so on..
 */
#[derive(Debug)]
pub struct MultiIndexingFace {
    pub vertices: Vec<usize>, // indices of the vertices
    pub normal_indices: Vec<usize>,
    pub tex_coord_indices: Vec<usize>,
    pub smoothing_group: Option<u32>,
}

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub real_verts : Vec<ObjLoaderRealtimeVertex>,
    pub faces: Vec<u32>,
    pub feces: Vec<Face>,
}

impl Mesh {
    pub fn new() -> Self {
        Mesh {
            vertices: Vec::new(),
            faces: Vec::new(),
            feces: Vec::new(),
            real_verts: Vec::new()
        }
    }
}

use std::fs::File;
use std::io::{BufRead, BufReader};
use bytemuck::{Pod, Zeroable};

impl Mesh {
    pub fn parse_from_file(path: &str) -> Result< HashMap<String,Mesh>, Box<dyn std::error::Error>> {

        //A good start, but it does not actually do anything with the smoothing group
        //And a Face here contains indices into each data array, i.e,
        //Into Position, into Normals, into texture coords
        //But our realtime mesh will only have ONE index
        //into a vertex format that has all needed things associated with it.


        //Also - now I am using a nalgebra, but I guess - just embrace all the libs until I
        //find time or energy to start writing own stuff for fun...


        let file = File::open(path)?;
        let reader = BufReader::new(file);


        let mut final_data: HashMap<String,Mesh> = HashMap::new();
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut tex_coords = Vec::new();
        let mut faces = Vec::new();
        let mut current_smoothing_group = None;
        let mut object_name : String = String::new();

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.is_empty() {
                continue;
            }

            match parts[0] {

                "o" => {
                    // object start
                    if !vertices.is_empty(){
                        let mesh = Self::create_realtime_mesh_from_loaded_data(&faces,&vertices,&normals,&tex_coords);
                        final_data.insert(object_name,mesh.unwrap());
                        faces.clear();
                        current_smoothing_group = None;
                    }
                    object_name = String::from(parts[1]).clone();
                }
                "v" => {
                    // Vertex position
                    let x: f32 = parts[1].parse()?;
                    let y: f32 = parts[2].parse()?;
                    let z: f32 = parts[3].parse()?;
                    vertices.push(BVec3(Vector3::new(x, y, z)));
                }
                "vn" => {
                    // Vertex normal
                    let x: f32 = parts[1].parse()?;
                    let y: f32 = parts[2].parse()?;
                    let z: f32 = parts[3].parse()?;
                    normals.push(BVec3(Vector3::new(x, y, z)));
                }
                "vt" => {
                    // Vertex texture coordinate
                    let u: f32 = parts[1].parse()?;
                    let v: f32 = parts[2].parse()?;
                    tex_coords.push(BVec2(Vector2::new(u,v)));
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

                    faces.push(MultiIndexingFace {
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
        let mesh = Self::create_realtime_mesh_from_loaded_data(&faces,&vertices,&normals,&tex_coords);
        final_data.insert(object_name,mesh.unwrap());
        Ok(final_data)
    }

    pub fn create_realtime_mesh_from_loaded_data(faces : &Vec<MultiIndexingFace>,
                                                 positions : &Vec<BVec3>,
                                                 normals : &Vec<BVec3>,
                                                 texture_coords : &Vec<BVec2>)
                                                 -> Result<Self,Box<dyn std::error::Error>>{

        let mut realtime_vertices: Vec<Vertex> = Vec::new();
        let mut super_realtime_vertices: Vec<ObjLoaderRealtimeVertex> = Vec::new();
        let mut hits: HashMap<String,u32> = HashMap::new();
        let mut realtime_faces: Vec<u32> = Vec::new();
        let mut realtime_feces: Vec<Face> = Vec::new();
        for face in faces{

            let mut i = 0;
            let mut new_face : Face = Face { face_indices: [0,0,0] };
            while i < face.vertices.len(){
                let key: String = format!("{}/{}/{}",face.vertices[i],face.tex_coord_indices[i],face.normal_indices[i]);
                match hits.get(&key){
                    Some(index) => {
                        realtime_faces.push(*index);
                        new_face.face_indices[i] = *index as u32;
                    },
                    None => {

                        //Collect all Vertex data
                        let vertex : Vertex = Vertex{
                            position: positions[face.vertices[i]],
                            normal: normals[face.normal_indices[i]],
                            tex_coord: texture_coords[face.tex_coord_indices[i]],
                        };

                        let realtime_vertex : ObjLoaderRealtimeVertex = ObjLoaderRealtimeVertex {
                            position : [vertex.position.0.x,vertex.position.0.y,vertex.position.0.z],
                            normal : [vertex.normal.0.x,vertex.normal.0.y,vertex.normal.0.z],
                            uv : [vertex.tex_coord.0.x,vertex.tex_coord.0.y]
                        };

                        let new_index = realtime_vertices.len() as u32;
                        realtime_vertices.push(vertex);
                        super_realtime_vertices.push(realtime_vertex);
                        realtime_faces.push(new_index);
                        new_face.face_indices[i] = new_index as u32;
                        hits.insert(key,new_index);
                    }
                }
                i+=1;
            }
            realtime_feces.push(new_face);
        }

        Ok(Mesh {
            vertices : realtime_vertices,
            faces  : realtime_faces ,
            feces : realtime_feces,
            real_verts: super_realtime_vertices,

        })

    }
}
