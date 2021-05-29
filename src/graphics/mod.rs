use std::{borrow::Cow, rc::Rc};

use glam::{Mat4, Vec3};

use wgpu::util::DeviceExt;

use bytemuck::{Pod, Zeroable};

use crate::transform::Transform;

#[derive(Debug, Clone)]
pub struct Material {
    pub normal: Option<Texture>,
    pub diffuse: Option<Texture>,
    pub base_diffuse_color: [f32; 4],
}

pub struct Frame {
    viewport: (i32, i32, i32, i32),
}

impl Frame {
    pub fn start(clear_color: [f32; 3], window_dimensions: (i32, i32)) -> Frame {
        unsafe {
            // gl::ClearColor(clear_color[0], clear_color[1], clear_color[2], 1.0);
            // gl::ClearDepth(1.0);
            // gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
        Frame {
            viewport: (0, 0, window_dimensions.0, window_dimensions.1),
        }
    }

    pub fn render(&self, primitive: &Mesh) {
        unsafe {
            todo!()
            // // gl::Enable(gl::BLEND);
            // // gl::BlendEquation(gl::FUNC_ADD);
            // // gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            // gl::Enable(gl::DEPTH_TEST);
            // gl::DepthFunc(gl::LEQUAL);
            // gl::Disable(gl::CULL_FACE); // CullClockwise

            // gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            // gl::Viewport(
            //     self.viewport.0,
            //     self.viewport.1,
            //     self.viewport.2,
            //     self.viewport.3,
            // );

            // shader.bind();

            // let vao: &u32 = &primitive.vao;
            // gl::BindVertexArray(*vao);
            // gl::DrawElements(
            //     gl::TRIANGLES,
            //     primitive.element_count,
            //     gl::UNSIGNED_SHORT,
            //     std::ptr::null(),
            // );
        }
    }
}

pub struct GraphicsContext {
    surface: wgpu::Surface,
    pub device: wgpu::Device, // todo! not pub
    swap_chain: wgpu::SwapChain,
    swap_chain_descriptor: wgpu::SwapChainDescriptor,
    pub queue: wgpu::Queue, // todo! not pub
    pipeline: wgpu::RenderPipeline,
}

impl GraphicsContext {
    pub async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::all());
        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate graphics adapter.");

        let adapter_info = adapter.get_info();
        println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to acquire GPU device.");

        let swapchain_format = adapter
            .get_swap_chain_preferred_format(&surface)
            .expect("Surface is not compatible with graphics adapter.");

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // let mx_ref: &[f32; 16] = &[
        //     1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 1.0,
        // ];
        // let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Uniform Buffer"),
        //     contents: bytemuck::cast_slice(mx_ref),
        //     usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        // });
        //
        // let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &bind_group_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: uniform_buf.as_entire_binding(),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::TextureView(&texture_view),
        //         },
        //     ],
        //     label: None,
        // });

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shader/shader.wgsl"))),
            flags: wgpu::ShaderFlags::all(),
        });

        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 4 * 4,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 4 * 7,
                    shader_location: 2,
                },
            ],
        }];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[swapchain_format.into()],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        Self {
            surface,
            device,
            swap_chain,
            swap_chain_descriptor,
            queue,
            pipeline: render_pipeline,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.swap_chain_descriptor.width = width;
        self.swap_chain_descriptor.height = height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    }

    pub fn render_scene<'gfx>(
        &'gfx self,
        scene: &'gfx crate::Scene,
        frame: &wgpu::SwapChainFrame,
        perspective: &Mat4,
        view: &Mat4,
        rpass: &mut wgpu::RenderPass<'gfx>
    ) {
        let base_transform = &scene.transform;
        for node in &scene.nodes {
            if node.meshes.is_empty() {
                continue;
            }

            let transform = {
                let mut current = node;
                let mut transform = node.transform.mat4().clone();
                'transform: loop {
                    current = match current.parent {
                        Some(index) => &scene.nodes[usize::from(index)],
                        None => break 'transform,
                    };

                    transform = transform.mul_mat4(current.transform.mat4());
                }
                Transform::from(transform)
            };

            for mesh in &node.meshes {
                let material = &mesh.material;
                let diffuse = material.diffuse.as_ref();
                let normal = material.normal.as_ref();

                let base_transform = base_transform.mat4().clone();
                let mesh_transform = transform.mat4().clone();
                let model = (mesh_transform * base_transform).to_cols_array_2d();

                // self.shader
                //     .uniform("perspective", perspective.to_cols_array_2d());
                // self.shader.uniform("view", view.to_cols_array_2d());
                // self.shader.uniform("model", model);
                // self.shader.uniform(
                //     "diffuse_texture",
                //     diffuse.cloned().unwrap_or(graphics::Texture::empty()),
                // );
                // self.shader.uniform(
                //     "normal_texture",
                //     normal.cloned().unwrap_or(graphics::Texture::empty()),
                // );
                // self.shader
                //     .uniform("has_diffuse_texture", diffuse.is_some());
                // self.shader.uniform("has_normal_texture", normal.is_some());
                // self.shader
                //     .uniform("base_diffuse_color", material.base_diffuse_color);
                // self.shader
                //     .uniform("u_light_direction", [-1.0, 0.4, 0.9f32]);
                // self.shader.uniform("shaded", true);

                rpass.set_index_buffer(mesh.inner.index.slice(..), wgpu::IndexFormat::Uint16);
                rpass.set_vertex_buffer(0, mesh.inner.vertex.slice(..));
                rpass.pop_debug_group();
                rpass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
                

                // frame.render(mesh, &self.shader);
            }
        }
    }

    pub fn render_billboard(
        &mut self,
        texture: &Texture,
        frame: &wgpu::SwapChainFrame,
        position: Vec3,
        perspective: &Mat4,
        camera: &crate::camera::Camera,
    ) {
        todo!()
        // let positions = [
        //     [-1.0, -1.0, 0.0],
        //     [1.0, -1.0, 0.0],
        //     [-1.0, 1.0, 0.0],
        //     [1.0, 1.0, 0.0],
        // ];
        // let normals = [
        //     [1.0, 0.0, 0.0],
        //     [1.0, 0.0, 0.0],
        //     [1.0, 0.0, 0.0],
        //     [1.0, 0.0, 0.0],
        // ];
        // let uvs = [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
        // let indices = [0, 1, 2, 3, 2, 1];
        // let material = Material {
        //     base_diffuse_color: [1.0, 1.0, 1.0, 1.0],
        //     diffuse: None,
        //     normal: None,
        // };
        // let mesh = Mesh::new(&positions, &normals, &uvs, &indices, &material);

        // let w = texture.width() as f32;
        // let h = texture.height() as f32;
        // let scale = Vec3::new(w / w.max(h) * 10.0, h / w.max(h) * 10.0, 1.0);
        // let rotation = {
        //     let fwd = camera.transform().position() - position;
        //     let fwd = -fwd.normalize().cross(GLOBAL_UP.into()).normalize();
        //     let yaw = f32::atan2(fwd.z, fwd.x);
        //     let pitch = f32::asin(fwd.y);
        //     Mat4::from_euler(glam::EulerRot::YXZ, -yaw, pitch, 0.0)
        // };
        // let model = Mat4::from_translation(position) * rotation * Mat4::from_scale(scale);

        // self.shader
        //     .uniform("perspective", perspective.to_cols_array_2d());
        // self.shader
        //     .uniform("view", camera.view().to_cols_array_2d());
        // self.shader.uniform("model", model.to_cols_array_2d());
        // self.shader.uniform("diffuse_texture", texture.clone());
        // self.shader.uniform("has_diffuse_texture", true);
        // self.shader.uniform("has_normal_texture", false);
        // self.shader.uniform("shaded", false);

        // frame.render(&mesh, &self.shader);
    }

    pub fn create_mesh(&self, vertices: &[Vertex], indices: &[u16], material: &Material) -> Mesh {
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsage::VERTEX,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsage::INDEX,
            });

        let inner = MeshStorage {
            vertex: vertex_buffer,
            index: index_buffer,
        };

        Mesh {
            inner: inner.into(),
            index_count: indices.len(),
            material: material.clone(),
        }
    }

    pub fn create_texture(
        &self,
        texels: &[u8],
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Texture {
        let texture_extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            texels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(std::num::NonZeroU32::new(width * 4).unwrap()),
                rows_per_image: None,
            },
            texture_extent,
        );

        Texture {
            texture: texture.into(),
            width,
            height,
        }
    }

    pub fn get_current_frame(&mut self) -> wgpu::SwapChainFrame {
        let frame = match self.swap_chain.get_current_frame() {
            Ok(frame) => frame,
            Err(_) => {
                self.swap_chain = self
                    .device
                    .create_swap_chain(&self.surface, &self.swap_chain_descriptor);
                self.swap_chain
                    .get_current_frame()
                    .expect("Failed to acquire next swap chain texture!")
            }
        };
        frame
    }

    pub fn begin_render_pass<'gfx>(&'gfx self, frame: &'gfx wgpu::SwapChainFrame, encoder: &'gfx mut wgpu::CommandEncoder) -> wgpu::RenderPass<'gfx> {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &frame.output.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        rpass.push_debug_group("Prepare data for draw...");
        rpass.set_pipeline(&self.pipeline);
        // todo! rpass.set_bind_group(0, &self.bind_group, &[]);

        rpass
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

#[derive(Debug)]
pub struct MeshStorage {
    pub vertex: wgpu::Buffer,
    pub index: wgpu::Buffer,
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub inner: Rc<MeshStorage>,
    pub index_count: usize,
    pub material: Material,
}

#[derive(Debug, Clone)]
pub struct Texture {
    pub texture: Rc<wgpu::Texture>,
    pub width: u32,
    pub height: u32,
}
