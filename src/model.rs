use anyhow::Result;
use glam::{vec2, vec3};
use wgpu::util::DeviceExt as _;
use std::{collections::HashMap, fs::File, io::BufReader};

use crate::{texture, utils::Vertex};


pub struct Model {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub meshes: Vec<SubMeshData>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct SubMeshData {
    pub name: String,
    pub index_offset: usize,
    pub index_count: usize,
    pub material: usize,
}


impl Model {
    pub fn new(
        device: &wgpu::Device,
        name: &str, 
        vertices: &Vec<Vertex>, 
        indices: &Vec<u32>,
        material: Material,
    ) -> Self {
        let vertex_as_byte_slice = bytemuck::cast_slice(vertices.as_slice());
        let indices_as_byte_slice = bytemuck::cast_slice(indices.as_slice());

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(format!("{name} Vertex Buffer").as_str()),
            contents: vertex_as_byte_slice,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(format!("{name} Index Buffer").as_str()),
            contents: indices_as_byte_slice,
            usage: wgpu::BufferUsages::INDEX,
        });

        let materials = vec![material];

        let sub_mesh_data = vec![
            SubMeshData{ 
                name: "solo".to_string(), 
                index_offset: 0, 
                index_count: indices.len(), 
                material: 0,
            }
        ];

        Self { 
            name: name.to_string(), 
            vertex_buffer: vertex_buf, 
            index_buffer: index_buf, 
            meshes: sub_mesh_data, 
            materials: materials
         }
    }

    pub fn load_model(
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        file_name: &str,
        bind_group_layout: &wgpu::BindGroupLayout
    ) ->Result<Self> {
        let file_path = std::path::Path::new(file_name);
        let mut reader = BufReader::new(File::open(file_name)?);

        let (models, obj_materials) = tobj::load_obj_buf(
            &mut reader,
            &tobj::LoadOptions { triangulate: true, ..Default::default() },
            |filename_mtl| {
                let full_path = get_file_relative_to(filename_mtl, file_path);
                let file = File::open(full_path).unwrap();
                let mut mtl_reader = BufReader::new(file);
                tobj::load_mtl_buf(&mut mtl_reader)
            },
        )?;

        let mut materials = Vec::<Material>::new();
        for m in obj_materials? {
            let texture_path = get_file_relative_to(std::path::Path::new(&m.diffuse_texture), file_path);

            let texture = texture::Texture::load_texture(device, queue, &texture_path.to_str().unwrap())?;
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: None,
            });
            materials.push(Material{
                name: m.name,
                diffuse_texture: texture,
                bind_group: bind_group,
            });
        }

        let mut indices = Vec::<u32>::new();
        let mut vertices = Vec::<Vertex>::new();
        let mut sub_mesh_datas = Vec::<SubMeshData>::new();
        let mut unique_vertices = HashMap::new();

        for model in &models {
            let index_offset = indices.len();
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
            sub_mesh_datas.push(SubMeshData{
                name: model.name.clone(),
                index_offset: index_offset as _,
                index_count: model.mesh.indices.len(),
                material: model.mesh.material_id.unwrap(),
            });
        }

        let vertex_as_byte_slice = bytemuck::cast_slice(vertices.as_slice());
        let indices_as_byte_slice = bytemuck::cast_slice(indices.as_slice());

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(format!("{file_name} Vertex Buffer").as_str()),
            contents: vertex_as_byte_slice,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(format!("{file_name} Index Buffer").as_str()),
            contents: indices_as_byte_slice,
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self { 
            name: file_name.to_string(), 
            vertex_buffer: vertex_buf, 
            index_buffer: index_buf, 
            meshes: sub_mesh_datas, 
            materials: materials
         })
    }

    pub fn get_material(&mut self,i:usize) -> &mut Material {
        &mut self.materials[i]
    }
}

fn get_file_relative_to(filename_mtl: &std::path::Path , file_path: &std::path::Path) -> std::path::PathBuf {
    let full_path = if let Some(parent) = file_path.parent() {
        parent.join(filename_mtl)
    } else {
        filename_mtl.to_owned()
    };
    full_path
}

pub trait ModelDrawer {
    fn draw_model(&mut self, cprojection_bind_group: &wgpu::BindGroup, camera_transform: &wgpu::BindGroup, model: &Model, model_transform: &wgpu::BindGroup);
}

impl<'rp> ModelDrawer for wgpu::RenderPass<'rp>{
    fn draw_model(&mut self, projection_bind_group: &wgpu::BindGroup, camera_transform: &wgpu::BindGroup, model: &Model, model_transform: &wgpu::BindGroup) {
        
        
        
        self.set_bind_group(0, projection_bind_group, &[]);
        self.set_bind_group(1, camera_transform, &[]);
        self.set_bind_group(2, model_transform, &[]);
        self.set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_vertex_buffer(0, model.vertex_buffer.slice(..));
        for meshdata in &model.meshes  {
            self.set_bind_group(3, &model.materials[meshdata.material].bind_group, &[]);
            self.draw_indexed(meshdata.index_offset as _ ..meshdata.index_count as u32, 0, 0..1);
        }
    }
}