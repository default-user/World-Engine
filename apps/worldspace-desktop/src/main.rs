use anyhow::Result;
use clap::Parser;
use egui::Context as EguiContext;
use glam::Vec3;
use std::sync::Arc;
use std::time::Instant;
use tracing_subscriber::EnvFilter;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};
use worldspace_author::Editor;
use worldspace_common::{EntityId, Transform};
use worldspace_ecs::{ComponentStore, MaterialHandle, MeshHandle, Renderable};
use worldspace_kernel::World;
use worldspace_persist::WorldStore;
use worldspace_render_wgpu::{FlyCamera, WgpuRenderer};
use worldspace_stream::GridPartition;
use worldspace_tools::WorldInspector;

#[derive(Parser)]
#[command(name = "worldspace-desktop", about = "Worldspace desktop application")]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// World data directory
    #[arg(long, default_value = "./world_data")]
    data_dir: String,
}

/// Application state.
struct AppState {
    world: World,
    editor: Editor,
    components: ComponentStore,
    camera: FlyCamera,
    grid: GridPartition,
    selected: Option<EntityId>,
    show_inspector: bool,
    data_dir: String,
    // Input state
    keys_held: std::collections::HashSet<KeyCode>,
    mouse_captured: bool,
    last_frame: Instant,
    // Fixed timestep
    tick_accumulator: f64,
    tick_rate: f64,
}

impl AppState {
    fn new(data_dir: String) -> Self {
        let mut world = World::with_seed(42);
        let mut editor = Editor::new();
        let mut components = ComponentStore::new();

        // Spawn initial entities
        let id1 = editor.spawn(&mut world, Transform::default());
        components.set_name(id1, "Origin Cube".into());
        components.set_renderable(
            id1,
            Renderable {
                mesh: MeshHandle(0),
                material: MaterialHandle(0),
            },
        );

        let id2 = editor.spawn(
            &mut world,
            Transform {
                position: Vec3::new(3.0, 0.0, 0.0),
                ..Transform::default()
            },
        );
        components.set_name(id2, "Red Cube".into());
        components.set_renderable(
            id2,
            Renderable {
                mesh: MeshHandle(0),
                material: MaterialHandle(1),
            },
        );

        let id3 = editor.spawn(
            &mut world,
            Transform {
                position: Vec3::new(-3.0, 0.0, 3.0),
                ..Transform::default()
            },
        );
        components.set_name(id3, "Blue Cube".into());
        components.set_renderable(
            id3,
            Renderable {
                mesh: MeshHandle(0),
                material: MaterialHandle(2),
            },
        );

        let mut grid = GridPartition::new(16.0);
        grid.rebuild(&world);

        Self {
            world,
            editor,
            components,
            camera: FlyCamera::default(),
            grid,
            selected: None,
            show_inspector: true,
            data_dir,
            keys_held: std::collections::HashSet::new(),
            mouse_captured: false,
            last_frame: Instant::now(),
            tick_accumulator: 0.0,
            tick_rate: 1.0 / 60.0,
        }
    }

    fn update(&mut self, dt: f32) {
        let speed_mult = if self.keys_held.contains(&KeyCode::ShiftLeft) {
            3.0
        } else {
            1.0
        };
        let dt_scaled = dt * speed_mult;

        if self.keys_held.contains(&KeyCode::KeyW) {
            self.camera.move_forward(dt_scaled);
        }
        if self.keys_held.contains(&KeyCode::KeyS) {
            self.camera.move_backward(dt_scaled);
        }
        if self.keys_held.contains(&KeyCode::KeyA) {
            self.camera.move_left(dt_scaled);
        }
        if self.keys_held.contains(&KeyCode::KeyD) {
            self.camera.move_right(dt_scaled);
        }
        if self.keys_held.contains(&KeyCode::Space) {
            self.camera.move_up(dt_scaled);
        }
        if self.keys_held.contains(&KeyCode::ControlLeft) {
            self.camera.move_down(dt_scaled);
        }

        // Fixed timestep for kernel ticking
        self.tick_accumulator += dt as f64;
        while self.tick_accumulator >= self.tick_rate {
            self.tick_accumulator -= self.tick_rate;
            // Kernel stepping at fixed rate (editor mode skips this)
        }

        self.grid.rebuild(&self.world);
    }

    fn handle_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            self.keys_held.insert(key);
        } else {
            self.keys_held.remove(&key);
        }

        if !pressed {
            return;
        }

        match key {
            KeyCode::KeyN => {
                let pos = self.camera.position + self.camera.forward() * 5.0;
                let id = self.editor.spawn(
                    &mut self.world,
                    Transform {
                        position: pos,
                        ..Transform::default()
                    },
                );
                self.components
                    .set_name(id, format!("Entity_{}", &id.0.to_string()[..8]));
                self.components.set_renderable(
                    id,
                    Renderable {
                        mesh: MeshHandle(0),
                        material: MaterialHandle(0),
                    },
                );
                self.selected = Some(id);
                tracing::info!("spawned entity {}", &id.0.to_string()[..8]);
            }
            KeyCode::Delete | KeyCode::Backspace => {
                if let Some(id) = self.selected {
                    if self.editor.despawn(&mut self.world, id).is_ok() {
                        self.components.remove_entity(id);
                        self.selected = None;
                        tracing::info!("deleted entity");
                    }
                }
            }
            KeyCode::KeyZ if self.keys_held.contains(&KeyCode::ControlLeft) => {
                if self.editor.undo(&mut self.world) {
                    tracing::info!("undo");
                }
            }
            KeyCode::KeyY if self.keys_held.contains(&KeyCode::ControlLeft) => {
                if self.editor.redo(&mut self.world) {
                    tracing::info!("redo");
                }
            }
            KeyCode::F5 => {
                self.save_world();
            }
            KeyCode::F9 => {
                self.load_world();
            }
            KeyCode::F1 => {
                self.show_inspector = !self.show_inspector;
            }
            KeyCode::Escape => {
                self.selected = None;
            }
            _ => {}
        }
    }

    fn save_world(&mut self) {
        match WorldStore::open(&self.data_dir) {
            Ok(mut store) => {
                if let Err(e) = store.take_snapshot(&self.world) {
                    tracing::error!("failed to save snapshot: {e}");
                    return;
                }
                let events = self.world.drain_events();
                if let Err(e) = store.append_events(&events) {
                    tracing::error!("failed to save events: {e}");
                    return;
                }
                tracing::info!("world saved to {}", self.data_dir);
            }
            Err(e) => {
                tracing::error!("failed to open store: {e}");
            }
        }
    }

    fn load_world(&mut self) {
        match WorldStore::open(&self.data_dir) {
            Ok(store) => match store.load_latest() {
                Ok(loaded) => {
                    self.world = loaded;
                    self.editor = Editor::new();
                    self.selected = None;
                    self.grid.rebuild(&self.world);
                    tracing::info!("world loaded from {}", self.data_dir);
                }
                Err(e) => {
                    tracing::error!("failed to load world: {e}");
                }
            },
            Err(e) => {
                tracing::error!("failed to open store: {e}");
            }
        }
    }

    fn draw_ui(&mut self, ctx: &EguiContext) {
        if !self.show_inspector {
            return;
        }

        let summary = WorldInspector::summary(&self.world);

        egui::SidePanel::left("inspector")
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("World Engine");
                ui.separator();
                ui.label(format!("Tick: {}  Seed: {}", summary.tick, summary.seed));
                ui.label(format!("Entities: {}", summary.entity_count));
                ui.label(format!(
                    "Camera: ({:.1}, {:.1}, {:.1})",
                    self.camera.position.x, self.camera.position.y, self.camera.position.z
                ));
                ui.separator();

                ui.heading("Tools");
                if ui.button("Spawn Entity (N)").clicked() {
                    let pos = self.camera.position + self.camera.forward() * 5.0;
                    let id = self.editor.spawn(
                        &mut self.world,
                        Transform {
                            position: pos,
                            ..Transform::default()
                        },
                    );
                    self.components
                        .set_name(id, format!("Entity_{}", &id.0.to_string()[..8]));
                    self.components.set_renderable(
                        id,
                        Renderable {
                            mesh: MeshHandle(0),
                            material: MaterialHandle(0),
                        },
                    );
                    self.selected = Some(id);
                }
                if ui.button("Delete Selected (Del)").clicked() {
                    if let Some(id) = self.selected {
                        if self.editor.despawn(&mut self.world, id).is_ok() {
                            self.components.remove_entity(id);
                            self.selected = None;
                        }
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("Undo (Ctrl+Z)").clicked() {
                        self.editor.undo(&mut self.world);
                    }
                    if ui.button("Redo (Ctrl+Y)").clicked() {
                        self.editor.redo(&mut self.world);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Save (F5)").clicked() {
                        self.save_world();
                    }
                    if ui.button("Load (F9)").clicked() {
                        self.load_world();
                    }
                });
                ui.label(format!(
                    "Undo: {} / Redo: {}",
                    self.editor.undo_count(),
                    self.editor.redo_count()
                ));

                ui.separator();
                ui.heading("Entities");

                let entity_ids: Vec<EntityId> = self.world.entities().keys().copied().collect();
                for id in &entity_ids {
                    let name = self
                        .components
                        .get_name(*id)
                        .map(|n| n.0.clone())
                        .unwrap_or_else(|| id.0.to_string()[..8].to_string());
                    let is_selected = self.selected == Some(*id);
                    let label = if is_selected {
                        format!("> {name}")
                    } else {
                        name
                    };
                    if ui.selectable_label(is_selected, label).clicked() {
                        self.selected = Some(*id);
                    }
                }

                if let Some(id) = self.selected {
                    ui.separator();
                    ui.heading("Inspector");
                    // Copy transform to avoid holding an immutable borrow on self.world
                    let current_transform = self.world.get(id).map(|d| d.transform);
                    if let Some(transform) = current_transform {
                        let mut pos = [
                            transform.position.x,
                            transform.position.y,
                            transform.position.z,
                        ];
                        let old_pos = pos;
                        ui.label("Position:");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut pos[0]).prefix("X: ").speed(0.1),
                            );
                            ui.add(
                                egui::DragValue::new(&mut pos[1]).prefix("Y: ").speed(0.1),
                            );
                            ui.add(
                                egui::DragValue::new(&mut pos[2]).prefix("Z: ").speed(0.1),
                            );
                        });
                        if pos != old_pos {
                            let new_t = Transform {
                                position: Vec3::new(pos[0], pos[1], pos[2]),
                                ..transform
                            };
                            let _ = self.editor.set_transform(&mut self.world, id, new_t);
                        }

                        let mut scale = [
                            transform.scale.x,
                            transform.scale.y,
                            transform.scale.z,
                        ];
                        let old_scale = scale;
                        ui.label("Scale:");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut scale[0])
                                    .prefix("X: ")
                                    .speed(0.1),
                            );
                            ui.add(
                                egui::DragValue::new(&mut scale[1])
                                    .prefix("Y: ")
                                    .speed(0.1),
                            );
                            ui.add(
                                egui::DragValue::new(&mut scale[2])
                                    .prefix("Z: ")
                                    .speed(0.1),
                            );
                        });
                        if scale != old_scale {
                            let new_t = Transform {
                                scale: Vec3::new(scale[0], scale[1], scale[2]),
                                ..transform
                            };
                            let _ = self.editor.set_transform(&mut self.world, id, new_t);
                        }
                    }
                }

                ui.separator();
                ui.small("F1: Toggle Inspector | RMB: Look | WASD: Move");
            });
    }
}

struct GpuApp {
    state: AppState,
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    config: Option<wgpu::SurfaceConfiguration>,
    renderer: Option<WgpuRenderer>,
    egui_ctx: EguiContext,
    egui_winit: Option<egui_winit::State>,
    egui_renderer: Option<egui_wgpu::Renderer>,
}

impl GpuApp {
    fn new(data_dir: String) -> Self {
        Self {
            state: AppState::new(data_dir),
            window: None,
            surface: None,
            device: None,
            queue: None,
            config: None,
            renderer: None,
            egui_ctx: EguiContext::default(),
            egui_winit: None,
            egui_renderer: None,
        }
    }
}

impl ApplicationHandler for GpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("World Engine")
            .with_inner_size(PhysicalSize::new(1280u32, 720));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("find adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("worldspace_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        ))
        .expect("create device");

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        self.state.camera.aspect = size.width as f32 / size.height.max(1) as f32;

        let renderer = WgpuRenderer::new(&device, surface_format, size.width, size.height);

        let egui_winit = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1, false);

        self.window = Some(window);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.renderer = Some(renderer);
        self.egui_winit = Some(egui_winit);
        self.egui_renderer = Some(egui_renderer);

        tracing::info!(
            "GPU initialized with {} backend",
            adapter.get_info().backend.to_str()
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(egui_winit) = &mut self.egui_winit {
            let response = egui_winit.on_window_event(self.window.as_ref().unwrap(), &event);
            if response.consumed {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let (Some(surface), Some(device), Some(config)) =
                    (&self.surface, &self.device, &mut self.config)
                {
                    config.width = new_size.width.max(1);
                    config.height = new_size.height.max(1);
                    surface.configure(device, config);
                    self.state.camera.aspect =
                        config.width as f32 / config.height.max(1) as f32;
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(device, config.width, config.height);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                self.state
                    .handle_key(key, key_state == ElementState::Pressed);
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: btn_state,
                ..
            } => {
                self.state.mouse_captured = btn_state == ElementState::Pressed;
                if let Some(window) = &self.window {
                    let _ = window.set_cursor_visible(!self.state.mouse_captured);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.state.last_frame).as_secs_f32().min(0.1);
                self.state.last_frame = now;
                self.state.update(dt);

                let (Some(surface), Some(device), Some(queue)) =
                    (&self.surface, &self.device, &self.queue)
                else {
                    return;
                };

                let output = match surface.get_current_texture() {
                    Ok(t) => t,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        if let Some(config) = &self.config {
                            surface.configure(device, config);
                        }
                        return;
                    }
                    Err(e) => {
                        tracing::error!("surface error: {e}");
                        return;
                    }
                };

                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                if let Some(renderer) = &self.renderer {
                    renderer.render(
                        device,
                        queue,
                        &view,
                        &self.state.camera,
                        &self.state.world,
                        self.state.components.renderables(),
                        self.state.selected,
                    );
                }

                let raw_input = self
                    .egui_winit
                    .as_mut()
                    .unwrap()
                    .take_egui_input(self.window.as_ref().unwrap());
                let full_output = self.egui_ctx.run(raw_input, |ctx| {
                    self.state.draw_ui(ctx);
                });

                self.egui_winit.as_mut().unwrap().handle_platform_output(
                    self.window.as_ref().unwrap(),
                    full_output.platform_output,
                );

                let paint_jobs = self
                    .egui_ctx
                    .tessellate(full_output.shapes, full_output.pixels_per_point);

                let screen_descriptor = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [
                        self.config.as_ref().unwrap().width,
                        self.config.as_ref().unwrap().height,
                    ],
                    pixels_per_point: full_output.pixels_per_point,
                };

                {
                    let egui_renderer = self.egui_renderer.as_mut().unwrap();
                    for (id, image_delta) in &full_output.textures_delta.set {
                        egui_renderer.update_texture(device, queue, *id, image_delta);
                    }
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("egui_encoder"),
                        });
                    egui_renderer.update_buffers(
                        device,
                        queue,
                        &mut encoder,
                        &paint_jobs,
                        &screen_descriptor,
                    );
                    {
                        let mut pass = encoder
                            .begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("egui_pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                ..Default::default()
                            })
                            .forget_lifetime();
                        egui_renderer.render(&mut pass, &paint_jobs, &screen_descriptor);
                    }
                    queue.submit(std::iter::once(encoder.finish()));
                    for id in &full_output.textures_delta.free {
                        egui_renderer.free_texture(id);
                    }
                }

                output.present();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.state.mouse_captured {
                self.state.camera.rotate(delta.0 as f32, delta.1 as f32);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .init();

    tracing::info!("worldspace-desktop starting");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = GpuApp::new(cli.data_dir);
    event_loop.run_app(&mut app)?;

    Ok(())
}
