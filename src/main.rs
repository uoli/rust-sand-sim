#![allow(
    dead_code,
)]


mod wgpu_app;
mod model;
mod primitives;
mod texture;
mod utils;

use anyhow::Result;
use glam::{Mat4, Vec3};
use model::{Model, ModelDrawer};
use primitives::CpuTexture;
use utils::Vertex;
use std::cell::RefCell;
use std::{rc::Rc, sync::Arc};
use std::mem::size_of;
use winit::window::Window;
use winit_input_helper::WinitInputHelper;


struct MyApp {
    window: Arc<Window>,
    forward_depth: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    pipeline_wire: Option<wgpu::RenderPipeline>,
    projection_buffer: wgpu::Buffer,
    projection_bindgroup: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    camera_bindgroup: wgpu::BindGroup,
    quad_tex: primitives::CpuTexture,
    quad_uniform_bind_group: wgpu::BindGroup,
    quad_model: Rc<RefCell<Model>>,
    frame_timer: utils::FrameTime,
    aspect_ratio: f32,
    show_wire: bool,
}


impl MyApp {
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    fn create_depth_texture(
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
    ) -> wgpu::TextureView {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        });

        depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

}


impl crate::wgpu_app::App for MyApp {
    fn init(
        window: Arc<Window>,
        config: &wgpu::SurfaceConfiguration,
        _adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {


        let vertex_size = size_of::<Vertex>();
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float{filterable: true},
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let transform_matrix_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(size_of::<Mat4>() as _),
                    },
                    count: None,
                }
            ],
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &transform_matrix_bind_group_layout, //projection
                &transform_matrix_bind_group_layout, //view (camera)
                &transform_matrix_bind_group_layout, //model
                &texture_bind_group_layout
            ],
            push_constant_ranges: &[],
        });

        // Create other resources
        let aspect_ratio = config.width as f32 / config.height as f32;
        //let projection = utils::create_perspective_matrix(aspect_ratio);
        let projection = utils::create_iso_matrix(config.width as _, config.height as _);
        let (projection_buffer, projection_bindgroup) = utils::create_matrix_buffer_and_bind_group(device, "projection", &transform_matrix_bind_group_layout, &projection);

        let cam_pos = glam::Vec3::new(0.0, -100.0, 0.0);
        let cam_rot = glam::Quat::from_rotation_arc(glam::Vec3::new(0.0, 1.0, 0.0), (glam::Vec3::new(0.0, 0.0, 0.0) - cam_pos).normalize());
        let camera = utils::get_view_matrix(cam_pos, cam_rot);
        let (camera_buffer, camera_bindgroup) = utils::create_matrix_buffer_and_bind_group(device, "camera", &transform_matrix_bind_group_layout, &camera);

        let wgsl_shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/shader.wgsl"));

        let vertex_buffer_layout = [wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { //pos
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute { //color
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 4 * 3,
                    shader_location: 1,
                },
                wgpu::VertexAttribute { //uv
                    format: wgpu::VertexFormat::Float32x2,
                    offset: (4 * 3) + (4 * 3),
                    shader_location: 2,
                },
            ],
        }];

        let quad_width = config.width as _;
        let quad_height = config.height as _;
        let quad_transform_matrix = glam::Mat4::from_translation(Vec3::new(0.0,0.0, 0.0));
        let (_, quad_uniform_bind_group) = utils::create_matrix_buffer_and_bind_group(device, "quad", &transform_matrix_bind_group_layout, &quad_transform_matrix);
        let quad_tex = primitives::CpuTexture::new(
            config.width as _,
            config.height as _,
            utils::new_texture(config.width as _, config.height as _));
        let material = primitives::create_custom_tex_material(device, queue, &texture_bind_group_layout, &quad_tex );
        let quad_model = std::rc::Rc::new(std::cell::RefCell::new(primitives::Quad::new(device, &glam::Vec2::new(quad_width,quad_height), material)));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &wgsl_shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &vertex_buffer_layout,
            },
            fragment: Some(wgpu::FragmentState {
                module: &wgsl_shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(config.view_formats[0].into())],
            }),
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Self::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let pipeline_wire = if device
            .features()
            .contains(wgpu::Features::POLYGON_MODE_LINE)
        {
            let pipeline_wire = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &wgsl_shader,
                    entry_point: "vs_main",
                    compilation_options: Default::default(),
                    buffers: &vertex_buffer_layout,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &wgsl_shader,
                    entry_point: "fs_wire",
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.view_formats[0],
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                operation: wgpu::BlendOperation::Add,
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            },
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Line,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Self::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
            Some(pipeline_wire)
        } else {
            None
        };

        let forward_depth = Self::create_depth_texture(config, device);

        let frame_timer = utils::FrameTime::new();


        // Done
        MyApp {
            window,
            forward_depth,
            pipeline,
            pipeline_wire,
            projection_buffer,
            projection_bindgroup,
            camera_buffer,
            camera_bindgroup,
            quad_tex,
            quad_uniform_bind_group,
            quad_model,
            aspect_ratio,
            frame_timer,
            show_wire: false,
        }
    }

    fn resize(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.aspect_ratio = config.width as f32 / config.height as f32;
        //let new_proj_mat = utils::create_perspective_matrix(self.aspect_ratio);
        let new_proj_mat = utils::create_iso_matrix(config.width as _, config.height as _);
        let mx_ref: &[f32; 16] = new_proj_mat.as_ref();
        queue.write_buffer(&self.projection_buffer, 0, bytemuck::cast_slice(mx_ref));
        self.forward_depth = Self::create_depth_texture(config, device);
    }

    fn process_event(&mut self, _event: &winit::event::Event<()>) {
        
    }

    fn update(&mut self, input: &WinitInputHelper) {
        let dt = self.frame_timer.tick();
        let _dt_as_sec = dt.as_secs_f32();

        const ZOOM_SPEED:f32 = 5.0;

        if input.mouse_pressed(winit::event::MouseButton::Left) || input.mouse_held(winit::event::MouseButton::Left) {
            if let Some((x,y)) = input.cursor() {
                spawn_sand_at(x as _, y as _, &mut self.quad_tex)
            }
        }
    }

    fn render(&mut self, view: &wgpu::TextureView, device: &wgpu::Device, queue: &wgpu::Queue) {

        self.quad_model.borrow_mut().get_material(0).diffuse_texture.set_pixels(queue,  &self.quad_tex.get_pixels()).expect("Unable to update the texture");
        

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment:  Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.forward_depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.push_debug_group("Prepare data for draw.");
            rpass.set_pipeline(&self.pipeline);
            rpass.pop_debug_group();
            rpass.insert_debug_marker("Draw!");


            rpass.draw_model(&self.projection_bindgroup, &self.camera_bindgroup, &self.quad_model.borrow(), &self.quad_uniform_bind_group);

        }

        queue.submit(Some(encoder.finish()));
    }
}

fn spawn_sand_at(x: usize, y: usize, tex: &mut CpuTexture) {
    let r = 0 as u8;
    let g = 255 as u8;
    let b = 255 as u8;
    let a = 255 as u8;
    tex.set_pixel(x, y, r, g, b, a);

}

fn main() {
    println!("Hello, world!");
    crate::wgpu_app::run::<MyApp>("My App");
}
