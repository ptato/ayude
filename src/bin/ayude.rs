use ayude::{
    graphics::{self, Material, Mesh},
    import_gltf,
    transform::Transform,
    Scene,
};
use glam::{Mat4, Vec3};
use glutin::{Api, ContextBuilder, GlProfile, GlRequest, Robustness, dpi::LogicalSize, event::{DeviceEvent, ElementState, Event, VirtualKeyCode, WindowEvent}, event_loop::{ControlFlow, EventLoop}, window::{Window, WindowBuilder}};
use image::EncodableLayout;
use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

const UP_VECTOR: [f32; 3] = [0.0, 1.0, 0.0];

pub struct World {
    camera_position: Vec3,
    camera_yaw: f32,   // radians
    camera_pitch: f32, // radians

    movement: [f32; 2], // stores WASD input

    the_scene: Scene,
    the_sphere: Scene,

    ricardo: graphics::Texture,

    rendering_skin: bool,

    renderer: WackyRenderer,
}

impl World {
    fn new() -> Self {
        static VERTEX_SOURCE: &str = include_str!("../resources/vertex.glsl");
        static FRAGMENT_SOURCE: &str = include_str!("../resources/fragment.glsl");
        let shader = graphics::Shader::from_sources(VERTEX_SOURCE, FRAGMENT_SOURCE).unwrap();

        let render_thing = WackyRenderer { shader };

        let gltf_file_name = "samples/knight/knight.gltf";
        // let gltf_file_name = "samples/principito_y_el_aviador/scene.gltf";
        let the_entity = import_gltf::import_default(gltf_file_name).unwrap();

        let the_sphere = import_gltf::import_default("samples/sphere.gltf").unwrap();

        let ricardo = {
            let file = std::fs::read("samples/ricardo.jpg").unwrap();
            let image = image::load_from_memory(&file).unwrap();
            let image = image.into_rgba();
            graphics::Texture::builder(
                image.as_bytes(),
                image.width() as u16,
                image.height() as u16,
                graphics::texture::TextureFormat::RGBA,
            )
            .build()
        };

        let world = World {
            camera_position: [0.0, 0.0, 37.0].into(),
            camera_yaw: std::f32::consts::PI,
            camera_pitch: 0.0,

            movement: [0.0, 0.0],

            the_scene: the_entity,
            the_sphere,

            ricardo,

            rendering_skin: false,

            renderer: render_thing,
        };

        world
    }

    fn update(&mut self, delta: Duration) {
        let forward_direction = Transform::from(Mat4::from_rotation_ypr(
            self.camera_yaw,
            self.camera_pitch,
            0.0,
        ))
        .forward();
        let right_direction = forward_direction.cross(UP_VECTOR.into()).normalize();

        let speed = 100.0;
        self.camera_position += forward_direction * self.movement[1] * speed * delta.as_secs_f32();
        self.camera_position += right_direction * self.movement[0] * speed * delta.as_secs_f32();
    }

    fn render(&mut self, window_dimensions: (i32, i32)) {
        let forward_direction = Transform::from(Mat4::from_rotation_ypr(
            self.camera_yaw,
            self.camera_pitch,
            0.0,
        ))
        .forward();
        let frame = graphics::Frame::start([0.1, 0.1, 0.1], window_dimensions);

        let perspective = glam::Mat4::perspective_rh_gl(
            std::f32::consts::PI / 3.0,
            window_dimensions.0 as f32 / window_dimensions.1 as f32,
            0.1,
            1024.0,
        );

        let view = glam::Mat4::look_at_rh(
            self.camera_position,
            self.camera_position + forward_direction,
            UP_VECTOR.into(),
        );

        {
            if !self.rendering_skin {
                self.renderer
                    .render_scene(&self.the_scene, &frame, &perspective, &view);
                let translation = Vec3::new(-1.0, -1.0, 0.0);
                self.renderer.render_billboard(
                    &self.ricardo,
                    &frame,
                    translation,
                    &perspective,
                    &view,
                );
            } else {
                let scene = &self.the_scene;
                for node in &scene.nodes {
                    let skin = match node.skin.as_ref() {
                        Some(skin) => skin,
                        None => continue,
                    };

                    let skeleton_transform = match skin.skeleton {
                        Some(skeleton) => Transform::from(
                            scene.nodes[usize::from(skeleton)].transform.mat4().clone(),
                        ),
                        None => scene.transform.clone(),
                    };

                    for &joint in &skin.joints {
                        let joint = &scene.nodes[usize::from(joint)];

                        let mut transform = joint.transform.mat4().clone();
                        let mut current = joint;
                        'transform: loop {
                            match current.parent {
                                Some(index) => current = &scene.nodes[usize::from(index)],
                                None => break 'transform,
                            }
                            transform = transform.mul_mat4(current.transform.mat4());
                        }

                        self.the_sphere.transform = Transform::from(
                            transform.mul_mat4(skeleton_transform.mat4())
                                * Mat4::from_scale(Vec3::new(0.25, 0.25, 0.25)),
                        );

                        self.renderer
                            .render_scene(&self.the_sphere, &frame, &perspective, &view);
                    }
                }
            };
        }
    }
}

pub struct WackyRenderer {
    shader: graphics::Shader,
}

impl WackyRenderer {
    fn render_scene(
        &mut self,
        scene: &Scene,
        frame: &graphics::Frame,
        perspective: &Mat4,
        view: &Mat4,
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

                self.shader
                    .uniform("perspective", perspective.to_cols_array_2d());
                self.shader.uniform("view", view.to_cols_array_2d());
                self.shader.uniform("model", model);
                self.shader.uniform(
                    "diffuse_texture",
                    diffuse.cloned().unwrap_or(graphics::Texture::empty()),
                );
                self.shader.uniform(
                    "normal_texture",
                    normal.cloned().unwrap_or(graphics::Texture::empty()),
                );
                self.shader
                    .uniform("has_diffuse_texture", diffuse.is_some());
                self.shader.uniform("has_normal_texture", normal.is_some());
                self.shader
                    .uniform("base_diffuse_color", material.base_diffuse_color);
                self.shader
                    .uniform("u_light_direction", [-1.0, 0.4, 0.9f32]);
                self.shader.uniform("shaded", true);

                frame.render(mesh, &self.shader);
            }
        }
    }

    fn render_billboard(
        &mut self,
        texture: &graphics::Texture,
        frame: &graphics::Frame,
        translation: Vec3,
        perspective: &Mat4,
        view: &Mat4,
    ) {
        let positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let normals = [
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ];
        let uvs = [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
        let indices = [0, 1, 2, 3, 2, 1];
        let material = Material {
            base_diffuse_color: [1.0, 1.0, 1.0, 1.0],
            diffuse: None,
            normal: None,
        };
        let mesh = Mesh::new(&positions, &normals, &uvs, &indices, &material);

        let w = texture.width() as f32;
        let h = texture.height() as f32;
        let scale = Vec3::new(w / w.max(h) * 10.0, h / w.max(h) * 10.0, 1.0);
        let model = Mat4::from_scale(scale) * Mat4::from_translation(translation);

        self.shader
            .uniform("perspective", perspective.to_cols_array_2d());
        self.shader.uniform("view", view.to_cols_array_2d());
        self.shader.uniform("model", model.to_cols_array_2d());
        self.shader.uniform("diffuse_texture", texture.clone());
        self.shader.uniform("has_diffuse_texture", true);
        self.shader.uniform("has_normal_texture", false);
        self.shader.uniform("shaded", false);

        frame.render(&mesh, &self.shader);
    }
}

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        // todo!
        // window.window().set_cursor_grab(false).unwrap();
        // window.window().set_cursor_visible(true);

        let mut lines = vec![];
        if let Some(message) = panic_info.payload().downcast_ref::<String>() {
            lines.push(message.to_string());
        }
        if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
            lines.push(message.to_string());
        }
        if let Some(location) = panic_info.location() {
            let loc = format!(
                "[{},{}] {}",
                location.line(),
                location.column(),
                location.file()
            );
            lines.push(loc);
        }
        msgbox::create("Error", &lines.join("\n"), msgbox::IconType::Error);
    }));

    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_title("a.yude")
        .with_inner_size(LogicalSize::new(1024.0, 768.0));
    let window = ContextBuilder::new()
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
        .with_gl_profile(GlProfile::Core)
        .with_gl_debug_flag(true)
        .with_gl_robustness(Robustness::RobustLoseContextOnReset)
        .with_vsync(true)
        .build_windowed(window_builder, &event_loop)
        .unwrap();
    let window = unsafe { window.make_current().unwrap() };

    window.window().set_cursor_grab(true).unwrap();
    window.window().set_cursor_visible(false);

    gl::load_with(|s| window.context().get_proc_address(s));

    let mut game = World::new();

    let mut previous_frame_time = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => return,
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    game.camera_yaw -= delta.0 as f32 / 150.0;
                    if game.camera_yaw >= 2.0 * PI {
                        game.camera_yaw -= 2.0 * PI;
                    }
                    if game.camera_yaw <= 0.0 {
                        game.camera_yaw += 2.0 * PI;
                    }

                    let freedom_y = 0.8;
                    game.camera_pitch += delta.1 as f32 / 150.0;
                    game.camera_pitch = game
                        .camera_pitch
                        .max(-PI / 2.0 * freedom_y)
                        .min(PI / 2.0 * freedom_y);
                }
                DeviceEvent::Key(input) => match input.virtual_keycode {
                    Some(VirtualKeyCode::W) => {
                        if input.state == ElementState::Pressed {
                            game.movement[1] = 1.0;
                        } else if input.state == ElementState::Released {
                            game.movement[1] = 0.0f32.min(game.movement[1]);
                        }
                    }
                    Some(VirtualKeyCode::A) => {
                        if input.state == ElementState::Pressed {
                            game.movement[0] = -1.0;
                        } else if input.state == ElementState::Released {
                            game.movement[0] = 0.0f32.max(game.movement[0]);
                        }
                    }
                    Some(VirtualKeyCode::S) => {
                        if input.state == ElementState::Pressed {
                            game.movement[1] = -1.0;
                        } else if input.state == ElementState::Released {
                            game.movement[1] = 0.0f32.max(game.movement[1]);
                        }
                    }
                    Some(VirtualKeyCode::D) => {
                        if input.state == ElementState::Pressed {
                            game.movement[0] = 1.0;
                        } else if input.state == ElementState::Released {
                            game.movement[0] = 0.0f32.min(game.movement[0]);
                        }
                    }
                    Some(VirtualKeyCode::Tab) if input.state == ElementState::Pressed => {
                        game.rendering_skin = !game.rendering_skin;
                    }
                    _ => return,
                },
                _ => return,
            },
            Event::MainEventsCleared => {
                let delta = previous_frame_time.elapsed();
                previous_frame_time = Instant::now();
                game.update(delta);
                window.window().request_redraw();
            }
            Event::RedrawRequested(..) => {
                game.render(get_window_dimensions(window.window()));
                window.swap_buffers().unwrap();
            }
            _ => return,
        }
    });
}

fn get_window_dimensions(window: &Window) -> (i32, i32) {
    let inner_size = window.inner_size();
    (inner_size.width as i32, inner_size.height as i32)
}