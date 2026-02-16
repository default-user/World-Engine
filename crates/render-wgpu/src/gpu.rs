use crate::camera::FlyCamera;
use crate::shaders;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use std::collections::BTreeMap;
use wgpu::util::DeviceExt;
use worldspace_common::EntityId;
use worldspace_ecs::Renderable;
use worldspace_kernel::World;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct InstanceData {
    model_0: [f32; 4],
    model_1: [f32; 4],
    model_2: [f32; 4],
    model_3: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GridVertex {
    position: [f32; 3],
    color: [f32; 4],
}

/// Generate unit cube vertices and indices.
fn cube_mesh() -> (Vec<Vertex>, Vec<u16>) {
    let p = 0.5_f32;
    #[rustfmt::skip]
    let vertices = vec![
        // +Z face
        Vertex { position: [-p, -p,  p], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [ p, -p,  p], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [ p,  p,  p], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [-p,  p,  p], normal: [0.0, 0.0, 1.0] },
        // -Z face
        Vertex { position: [ p, -p, -p], normal: [0.0, 0.0, -1.0] },
        Vertex { position: [-p, -p, -p], normal: [0.0, 0.0, -1.0] },
        Vertex { position: [-p,  p, -p], normal: [0.0, 0.0, -1.0] },
        Vertex { position: [ p,  p, -p], normal: [0.0, 0.0, -1.0] },
        // +X face
        Vertex { position: [ p, -p,  p], normal: [1.0, 0.0, 0.0] },
        Vertex { position: [ p, -p, -p], normal: [1.0, 0.0, 0.0] },
        Vertex { position: [ p,  p, -p], normal: [1.0, 0.0, 0.0] },
        Vertex { position: [ p,  p,  p], normal: [1.0, 0.0, 0.0] },
        // -X face
        Vertex { position: [-p, -p, -p], normal: [-1.0, 0.0, 0.0] },
        Vertex { position: [-p, -p,  p], normal: [-1.0, 0.0, 0.0] },
        Vertex { position: [-p,  p,  p], normal: [-1.0, 0.0, 0.0] },
        Vertex { position: [-p,  p, -p], normal: [-1.0, 0.0, 0.0] },
        // +Y face
        Vertex { position: [-p,  p,  p], normal: [0.0, 1.0, 0.0] },
        Vertex { position: [ p,  p,  p], normal: [0.0, 1.0, 0.0] },
        Vertex { position: [ p,  p, -p], normal: [0.0, 1.0, 0.0] },
        Vertex { position: [-p,  p, -p], normal: [0.0, 1.0, 0.0] },
        // -Y face
        Vertex { position: [-p, -p, -p], normal: [0.0, -1.0, 0.0] },
        Vertex { position: [ p, -p, -p], normal: [0.0, -1.0, 0.0] },
        Vertex { position: [ p, -p,  p], normal: [0.0, -1.0, 0.0] },
        Vertex { position: [-p, -p,  p], normal: [0.0, -1.0, 0.0] },
    ];
    #[rustfmt::skip]
    let indices: Vec<u16> = vec![
        0,1,2, 2,3,0,       // +Z
        4,5,6, 6,7,4,       // -Z
        8,9,10, 10,11,8,    // +X
        12,13,14, 14,15,12, // -X
        16,17,18, 18,19,16, // +Y
        20,21,22, 22,23,20, // -Y
    ];
    (vertices, indices)
}

/// Generate grid floor line vertices.
fn grid_mesh(half_extent: i32, spacing: f32) -> Vec<GridVertex> {
    let mut verts = Vec::new();
    let color = [0.4, 0.4, 0.4, 1.0];
    let extent = half_extent as f32 * spacing;

    for i in -half_extent..=half_extent {
        let offset = i as f32 * spacing;
        // Lines along X
        verts.push(GridVertex {
            position: [-extent, 0.0, offset],
            color,
        });
        verts.push(GridVertex {
            position: [extent, 0.0, offset],
            color,
        });
        // Lines along Z
        verts.push(GridVertex {
            position: [offset, 0.0, -extent],
            color,
        });
        verts.push(GridVertex {
            position: [offset, 0.0, extent],
            color,
        });
    }
    verts
}

/// wgpu-based world renderer.
pub struct WgpuRenderer {
    cube_pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    cube_vertex_buffer: wgpu::Buffer,
    cube_index_buffer: wgpu::Buffer,
    cube_index_count: u32,
    grid_vertex_buffer: wgpu::Buffer,
    grid_vertex_count: u32,
    instance_buffer: wgpu::Buffer,
    max_instances: u32,
    depth_texture: wgpu::TextureView,
    surface_format: wgpu::TextureFormat,
}

impl WgpuRenderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        // Uniform buffer
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: bytemuck::bytes_of(&Uniforms {
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Cube pipeline
        let cube_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cube_shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::WORLD_SHADER.into()),
        });

        let cube_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cube_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &cube_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x3,
                            1 => Float32x3,
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<InstanceData>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            2 => Float32x4,
                            3 => Float32x4,
                            4 => Float32x4,
                            5 => Float32x4,
                            6 => Float32x4,
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &cube_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // Grid pipeline
        let grid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("grid_shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::GRID_SHADER.into()),
        });

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &grid_shader,
                entry_point: Some("vs_grid"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GridVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x4,
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &grid_shader,
                entry_point: Some("fs_grid"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // Cube mesh
        let (cube_verts, cube_indices) = cube_mesh();
        let cube_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube_vertex_buffer"),
            contents: bytemuck::cast_slice(&cube_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let cube_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube_index_buffer"),
            contents: bytemuck::cast_slice(&cube_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let cube_index_count = cube_indices.len() as u32;

        // Grid mesh
        let grid_verts = grid_mesh(50, 1.0);
        let grid_vertex_count = grid_verts.len() as u32;
        let grid_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid_vertex_buffer"),
            contents: bytemuck::cast_slice(&grid_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Instance buffer (pre-allocated)
        let max_instances = 10_000u32;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            size: (max_instances as u64) * std::mem::size_of::<InstanceData>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let depth_texture = Self::create_depth_texture(device, width, height);

        Self {
            cube_pipeline,
            grid_pipeline,
            uniform_buffer,
            uniform_bind_group,
            cube_vertex_buffer,
            cube_index_buffer,
            cube_index_count,
            grid_vertex_buffer,
            grid_vertex_count,
            instance_buffer,
            max_instances,
            depth_texture,
            surface_format,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.depth_texture = Self::create_depth_texture(device, width, height);
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_format
    }

    /// Render one frame: grid floor + entity cubes.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        camera: &FlyCamera,
        world: &World,
        renderables: &BTreeMap<EntityId, Renderable>,
        selected: Option<EntityId>,
    ) {
        let vp = camera.view_projection();
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms {
                view_proj: vp.to_cols_array_2d(),
            }),
        );

        // Build instance data from entities
        let mut instances: Vec<InstanceData> = Vec::new();
        for (id, entity_data) in world.entities() {
            if instances.len() >= self.max_instances as usize {
                break;
            }
            let t = &entity_data.transform;
            let model = Mat4::from_scale_rotation_translation(t.scale, t.rotation, t.position);
            let cols = model.to_cols_array_2d();

            let is_renderable = renderables.contains_key(id);
            let is_selected = selected == Some(*id);

            let color = if is_selected {
                [1.0, 0.8, 0.0, 1.0] // Yellow for selected
            } else if is_renderable {
                [0.2, 0.6, 1.0, 1.0] // Blue for renderable
            } else {
                [0.7, 0.7, 0.7, 1.0] // Gray default
            };

            instances.push(InstanceData {
                model_0: cols[0],
                model_1: cols[1],
                model_2: cols[2],
                model_3: cols[3],
                color,
            });
        }

        if !instances.is_empty() {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instances),
            );
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Draw grid floor
            pass.set_pipeline(&self.grid_pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, self.grid_vertex_buffer.slice(..));
            pass.draw(0..self.grid_vertex_count, 0..1);

            // Draw entity cubes
            if !instances.is_empty() {
                pass.set_pipeline(&self.cube_pipeline);
                pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                pass.set_vertex_buffer(0, self.cube_vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                pass.set_index_buffer(
                    self.cube_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                pass.draw_indexed(0..self.cube_index_count, 0, 0..instances.len() as u32);
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        texture.create_view(&Default::default())
    }
}
