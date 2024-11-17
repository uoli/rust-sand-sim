use glam::{Vec2, Vec3};

use crate::{model, texture, utils::Vertex, Model};

pub struct Quad {

}

impl Quad {
    pub fn new(device: &wgpu::Device, size: &Vec2, material: model::Material) -> Model {
        let vertices = vec![
            Vertex::new(Vec3::new(0.0,    0.0, 0.0   ), Vec2::new(0.0, 0.0)),
            Vertex::new(Vec3::new(0.0,    0.0, size.y), Vec2::new(0.0, 1.0)),
            Vertex::new(Vec3::new(size.x, 0.0, size.y), Vec2::new(1.0, 1.0)),
            Vertex::new(Vec3::new(size.x, 0.0, 0.0   ), Vec2::new(1.0, 0.0))
        ];

        let indices = vec![
            0,1,2,
            2,3,0,
        ];

        //let material = create_white_material(device, queue, bind_group_layout );

        Model::new(device, "Quad", &vertices, &indices, material)
    }
}

pub struct CpuTexture {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl CpuTexture {
    pub fn new(width: usize, height: usize, data: Vec<u8>) -> Self {
        Self { width, height, data }
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, r:u8, g:u8, b:u8, a:u8) {
        let i = ((y * self.width  + x) * 4) as usize;
        self.data[i + 0] = r;
        self.data[i + 1] = g;
        self.data[i + 2] = b;
        self.data[i + 3] = a;
    }

    pub fn get_pixels(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn get_pixel(&self,x: usize, y: usize) -> (u8,u8,u8,u8) {
        let i = ((y * self.width  + x) * 4) as usize;
        let r = self.data[i + 0];
        let g = self.data[i + 1];
        let b = self.data[i + 2];
        let a = self.data[i + 3];
        (r,g,b,a)
    }
}

pub fn create_white_material(device: &wgpu::Device, queue: &wgpu::Queue, bind_group_layout: &wgpu::BindGroupLayout ) -> model::Material {
    let texture_data = vec![
        255,255,255,255,
        255,255,255,255,
        255,255,255,255,
        255,255,255,255,
    ];
    let cpu_texture = CpuTexture::new(2,2,texture_data);
    create_custom_tex_material(device, queue, bind_group_layout, &cpu_texture)
}

pub fn create_custom_tex_material(device: &wgpu::Device, queue: &wgpu::Queue, bind_group_layout: &wgpu::BindGroupLayout, cpu_texture: &CpuTexture ) -> model::Material {

        let texture = texture::Texture::from_bytes(device, queue, cpu_texture.width,cpu_texture.height, &cpu_texture.data).expect("Unable to create white texture");
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
        let diffuse_texture = texture;
        crate::model::Material{
            name: "White Material".to_string(),
            diffuse_texture,
            bind_group,
        }
}