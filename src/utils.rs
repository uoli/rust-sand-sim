use anyhow::Result;
use bytemuck::NoUninit;
use std::f32::consts;
use std::hash::{Hash, Hasher};
use std::{collections::HashMap, fs::File, io::BufReader};
use glam::{vec2, vec3, Vec2, Vec3, Quat};
use wgpu::util::DeviceExt;


pub(crate) unsafe fn slice_as_u8_slice<T: Sized>(any: &[T]) -> &[u8] {
    let ptr = (any as *const [T]) as *const u8;
    std::slice::from_raw_parts(ptr, any.len() * std::mem::size_of::<T>())
}

pub fn align_buffer_size(size: u64, alignment: u64) -> u64 {
    return (size + alignment - 1) & !(alignment - 1);
}

pub fn load_texture(file_name: &str) -> Result<(u32, u32, u64, Vec<u8>)> {
    let image = File::open(file_name)?;

    let decoder = png::Decoder::new(image);
    let mut reader = decoder.read_info()?;

    let mut pixels = vec![0;  reader.info().raw_bytes()];
    reader.next_frame(&mut pixels)?;

    let size = reader.info().raw_bytes() as u64;
    let (width, height) = reader.info().size();

    if /*width != 1024 || height != 1024 ||*/ reader.info().color_type != png::ColorType::Rgba {
        panic!("Invalid texture image.");
    }

    Ok((width, height, size, pixels))
}

pub fn load_model_data(indices: &mut Vec<u32>, vertices: &mut Vec<Vertex>) -> Result<()> {
    let mut reader = BufReader::new(File::open("resources/viking_room.obj")?);

    let (models, _) = tobj::load_obj_buf(
        &mut reader,
        &tobj::LoadOptions { triangulate: true, ..Default::default() },
        |_| Ok(Default::default()),
    )?;

    let mut unique_vertices = HashMap::new();

    for model in &models {
        for index in &model.mesh.indices {
            let pos_offset = (3 * index) as usize;
            let tex_coord_offset = (2 * index) as usize;

            let vertex = Vertex {
                pos: vec3(
                    model.mesh.positions[pos_offset],
                    model.mesh.positions[pos_offset + 1],
                    model.mesh.positions[pos_offset + 2],
                ),
                color: vec3(1.0, 1.0, 1.0),
                tex_coord: vec2(
                    model.mesh.texcoords[tex_coord_offset],
                    1.0 - model.mesh.texcoords[tex_coord_offset + 1],
                ),
            };

            if let Some(index) = unique_vertices.get(&vertex) {
                indices.push(*index as u32);
            } else {
                let index = vertices.len();
                unique_vertices.insert(vertex, index);
                vertices.push(vertex);
                indices.push(index as u32);
            }
    
        }
    }

    Ok(())
}


pub(crate) fn get_view_matrix(pos: Vec3, quat: Quat) -> glam::Mat4 {
    let center = quat * Vec3::new(0.0, 1.0, 0.0);
    glam::Mat4::look_at_rh(
        pos,
        center,
        glam::Vec3::Z,
    )
}

pub(crate) fn create_perspective_matrix(aspect_ratio: f32) -> glam::Mat4 {
    glam::Mat4::perspective_rh(consts::FRAC_PI_4, aspect_ratio, 1.0, 100.0)
}

pub(crate) fn create_iso_matrix(width: f32, height: f32) -> glam::Mat4 {
    glam::Mat4::orthographic_rh(0.0, width, height, 0.0, 1.0, 100.0)
}

pub(crate) fn create_matrix_buffer_and_bind_group(device: &wgpu::Device, label: &str, bind_group_layout: &wgpu::BindGroupLayout, matrix: &glam::Mat4) -> (wgpu::Buffer, wgpu::BindGroup) {
    let matrix_ref: &[f32; 16] = matrix.as_ref();
    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(format!("{label} Uniform Buffer").as_str()),
        contents: bytemuck::cast_slice(matrix_ref),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    
    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }
        ],
        label: None,
    });
    (uniform_buf, uniform_bind_group)
}

pub fn new_texture(width:usize, height: usize) -> Vec<u8> {
    let size = (width * height * 4) as _; //RGBA assumed
    let mut res = Vec::<u8>::with_capacity(size);
    res.resize(size, 255);

    for y  in  0..height {
        let v_ratio = y as f32 / height as f32; 
        let g = (v_ratio * 255.0).round() as u8;
        for x in 0..width {
            let i = (y*width + x) * 4;
            let h_ratio = x as f32 / width as f32;

            let r = (h_ratio * 255.0) as u8;
            //let g = (v_ratio * 255.0).round() as u8;
            let b = 255 as u8;
            let a = 255 as u8;
            
            res[i + 0] = r;
            res[i + 1] = g;
            res[i + 2] = b;
            res[i + 3] = a;
        }
    }


    res
}


#[repr(C)]
#[derive(Copy, Clone, Debug, NoUninit)]
pub struct Vertex {
    pub(crate) pos: Vec3,
    pub(crate) color: Vec3,
    pub(crate) tex_coord: Vec2,
}

impl Vertex {
    pub const fn new(pos: Vec3, tex_coord: Vec2) -> Self {
        let color = Vec3::new(1.0, 1.0, 1.0);
        Self { pos, color, tex_coord }
    }
}

impl PartialEq for Vertex {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
            && self.color == other.color
            && self.tex_coord == other.tex_coord
    }
}

impl Eq for Vertex {}

impl Hash for Vertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pos[0].to_bits().hash(state);
        self.pos[1].to_bits().hash(state);
        self.pos[2].to_bits().hash(state);
        self.color[0].to_bits().hash(state);
        self.color[1].to_bits().hash(state);
        self.color[2].to_bits().hash(state);
        self.tex_coord[0].to_bits().hash(state);
        self.tex_coord[1].to_bits().hash(state);
    }
}


pub struct FrameTime {
    last_frame_instant: std::time::Instant,
    last_frame_dt: std::time::Duration 
}

impl FrameTime {
    pub fn new() -> Self {
        FrameTime{
            last_frame_instant: std::time::Instant::now(),
            last_frame_dt: std::time::Duration::new(0, 0), 
        }
    }

    pub fn tick(&mut self) -> std::time::Duration {
        let now = std::time::Instant::now();
        self.last_frame_dt = now - self.last_frame_instant;
        self.last_frame_instant = now;
        self.last_frame_dt
    }

    pub fn get_dt(&self) -> std::time::Duration {
        self.last_frame_dt
    }
}