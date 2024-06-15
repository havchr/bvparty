use std::fs::File;
use std::io::{Cursor, Read,BufReader};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bvparty::run;
use bvparty::nocmp::spline_curves;
use rodio::{Decoder, OutputStream, source::Source};

extern crate lyon;
use lyon::math::{point, Point};
use lyon::path::Path;
use lyon::tessellation::*;

#[repr(C)]
#[derive(Debug,Copy,Clone,bytemuck::Pod,bytemuck::Zeroable)]
pub struct MidiHeader{
    pub mthd:i32,
    pub header_length:i32,
    pub format:i16,
    pub n:i16,
}

fn main() {

    println!("Hello, world!");



    // Build a Path.
    let mut builder = Path::builder();
    builder.begin(point(0.0, 0.0));
    builder.line_to(point(1.0, 0.0));
    builder.quadratic_bezier_to(point(2.0, 0.0), point(2.0, 1.0));
    builder.cubic_bezier_to(point(1.0, 1.0), point(0.0, 1.0), point(0.0, 0.0));
    builder.end(true);
    let path = builder.build();
    // Let's use our own custom vertex type instead of the default one.
    #[derive(Copy, Clone, Debug)]
    struct MyVertex { position: [f32; 2] };
    // Will contain the result of the tessellation.
    let mut geometry: VertexBuffers<MyVertex, u16> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();
    {
        // Compute the tessellation.
        tessellator.tessellate_path(
            &path,
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
                MyVertex {
                    position: vertex.position().to_array(),
                }
            }),
        ).unwrap();
    }
    // The tessellated geometry is ready to be uploaded to the GPU.
    println!(" -- {} vertices {} indices",
             geometry.vertices.len(),
             geometry.indices.len()
    );
    
    

    let midi_path = "art/midi_test.mid";

    let midi_header_size = std::mem::size_of::<MidiHeader>();



    //reading midi - https://www.ccarh.org/courses/253/handout/smf/
    // Open the file in read-only mode
    let mut midi_file = File::open(midi_path).expect("Failed to open file");
    let mut buffer = vec![];
    midi_file.read_to_end(&mut buffer).unwrap();
    let mut cursor = Cursor::new(&buffer);

    let value = cursor.read_i32::<BigEndian>().unwrap();
    let midi_mthd_check = 0x4d546864;
    println!("Read bytes: {:02X?}", value);
    if( value != midi_mthd_check){
        println!("Maybe this is not a midi file...");
    }
    else{

        println!("Found a midi file!!");
        let header_length = cursor.read_i32::<byteorder::BigEndian>().unwrap();
        if(header_length == 6){

            let format = cursor.read_i16::<byteorder::BigEndian>().unwrap();
            let num_track_chunks= cursor.read_i16::<byteorder::BigEndian>().unwrap();
            let division= cursor.read_i16::<byteorder::BigEndian>().unwrap();

            println!("midi format was {}",format);
            println!("num_track_chunks was {}",num_track_chunks);
            println!("division was {}",division);
            //next up - reading track chunks and track events
            //and also reading meta events..
        }
        else{
           println!("Expected header length to be 6, but was {}",header_length);
        }
    }

   //Try playing music.. 
    // Get an output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    // Load a sound from a file, using a path relative to Cargo.toml
    let file = BufReader::new(File::open("art/track.wav").unwrap());
    // Decode that sound file into a source
    let source = Decoder::new(file).unwrap();
    // Play the sound directly on the device
    stream_handle.play_raw(source.convert_samples());
    
    
    pollster::block_on(run());
}
