use ayude::{Catalog, Entity, graphics::{self}, import_gltf};
use glam::{Vec3};
use glutin::{
    dpi::LogicalSize,
    event::{DeviceEvent, ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    Api, ContextBuilder, GlProfile, GlRequest, Robustness,
};
use graphics::Mesh;
use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

fn calculate_forward_direction(yaw: f32, pitch: f32) -> Vec3 {
    let result: Vec3 = [
        (-yaw).cos() * pitch.cos(),
        (-yaw).sin() * pitch.cos(),
        pitch.sin(),
    ]
    .into();
    result.normalize()
}


pub struct World {
    camera_position: Vec3,
    camera_yaw: f32,
    camera_pitch: f32,

    movement: [f32; 2], // stores WASD input

    shader: graphics::Shader,

    meshes: Catalog<Mesh>,
    materials: Catalog<graphics::Material>,
    textures: Catalog<graphics::Texture>,
    entities: Catalog<Entity>,
}

impl World {
    fn new() -> Self {
        static VERTEX_SOURCE: &str = include_str!("../resources/vertex.glsl");
        static FRAGMENT_SOURCE: &str = include_str!("../resources/fragment.glsl");
        let shader = graphics::Shader::from_sources(VERTEX_SOURCE, FRAGMENT_SOURCE).unwrap();

        let mut world = World {
            camera_position: [0.0, 0.0, 0.0].into(),
            camera_yaw: 0.0,
            camera_pitch: 0.0,

            movement: [0.0, 0.0],

            shader,

            meshes: Catalog::new(),
            materials: Catalog::new(),
            textures: Catalog::new(),
            entities: Catalog::new(),
        };

        let gltf_file_name = "samples/knight/knight.gltf";
        import_gltf::import(gltf_file_name, &mut world.entities, &mut world.meshes, &mut world.materials, &mut world.textures);

        world
    }

    fn update(&mut self, delta: Duration) {
        let forward_direction = calculate_forward_direction(self.camera_yaw, self.camera_pitch);
        let right_direction = forward_direction.cross([0.0, 0.0, 1.0].into()).normalize();

        let speed = 100.0;
        self.camera_position += forward_direction * self.movement[1] * speed * delta.as_secs_f32();
        self.camera_position += right_direction * self.movement[0] * speed * delta.as_secs_f32();
    }

    fn render(&mut self, window_dimensions: (i32, i32)) {
        let forward_direction = calculate_forward_direction(self.camera_yaw, self.camera_pitch);
        let frame = graphics::Frame::start([0.0, 0.0, 1.0], window_dimensions);

        let perspective = glam::Mat4::perspective_rh_gl(
            std::f32::consts::PI / 3.0,
            window_dimensions.0 as f32 / window_dimensions.1 as f32,
            0.1,
            1024.0,
        );

        let view = glam::Mat4::look_at_rh(
            self.camera_position,
            self.camera_position + forward_direction,
            [0.0, 0.0, 1.0].into(),
        );

        for entity in self.entities.iter() {
            let model = entity.transform;
            if let Some(mesh) = self.meshes.get_opt(entity.mesh) {
                for primitive in &mesh.primitives {
                    let material = self.materials.get(primitive.material).expect("XD");
                    let diffuse = self.textures.get_opt(material.diffuse);
                    let normal = self.textures.get_opt(material.normal);

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

                    frame.render(primitive, &self.shader);
                }
            }
        }
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
                    game.camera_yaw += delta.0 as f32 * 0.006;
                    if game.camera_yaw >= 2.0 * PI {
                        game.camera_yaw -= 2.0 * PI;
                    }
                    if game.camera_yaw <= 0.0 {
                        game.camera_yaw += 2.0 * PI;
                    }

                    let freedom_y = 0.8;
                    game.camera_pitch -= delta.1 as f32 * 0.006;
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
                let inner_size = window.window().inner_size();
                game.render((inner_size.width as i32, inner_size.height as i32));
                window.swap_buffers().unwrap();
            }
            _ => return,
        }
    });
}
