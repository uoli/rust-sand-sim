use anyhow::Result;

use crate::utils;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: usize, 
        height: usize,
        data: &[u8]
    ) -> Result<Self> {
        
        let texture_extent = wgpu::Extent3d {
            width: width as _,
            height: height as _,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        queue.write_texture(
            texture.as_image_copy(),
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width as u32 * 4),
                rows_per_image: None,
            },
            texture_extent,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,    
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            lod_min_clamp: 0.0,
            lod_max_clamp: 1000.0,
            anisotropy_clamp: 1,
            ..Default::default()
        });
        Ok(Self{
            texture,
            view: texture_view,
            sampler,
        })
    }

    pub fn load_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        file_name: &str
    ) -> Result<Self> {
        let (width, height, _, data) = utils::load_texture(file_name)?;
        Self::from_bytes(device, queue, width as _, height as _, &data)
    }

    pub fn set_pixels(&self, queue: &wgpu::Queue, pixels: &[u8]) -> Result<()> {
        let texture_extent = wgpu::Extent3d {
            width: self.texture.size().width,
            height: self.texture.size().height,
            depth_or_array_layers: 1,
        };
        queue.write_texture(
            self.texture.as_image_copy(),
            pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.texture.size().width as u32 * 4),
                rows_per_image: None,
            },
            texture_extent,
        );
        Ok(())
    }
}