use glam::Vec3;
use granny_rs::entities;
use granny_rs::game::{self, Difficulty, World};
use granny_rs::render;
use granny_rs::render::{Renderer, Vertex};
use granny_rs::scene;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct App {
    state: Option<State>,
    difficulty: Option<Difficulty>,
}

struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    world: World,
    keys: Keys,
    last: std::time::Instant,
    mouse_dx: f32,
    mouse_dy: f32,
    mouse_captured: bool,
    last_cursor: (f32, f32),
}

#[derive(Default)]
struct Keys {
    turn_left: bool,
    turn_right: bool,
    fwd: bool,
    back: bool,
    left: bool,
    right: bool,
    run: bool,
    crouch: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Granny — Rust")
            .with_inner_size(LogicalSize::new(1280, 720));
        let window = Arc::new(el.create_window(attrs).unwrap());

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .unwrap();

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let mut renderer = Renderer::new(device, queue, config);

        if let Ok(img) = image::open("assets/atlas.png") {
            renderer.set_atlas(&img.to_rgba8());
        }

        let bytes = std::fs::read("assets/scene.bin").expect("assets/scene.bin");
        let scene = scene::Scene::load(&bytes).expect("scene");

        // Build a single merged vertex buffer with normals.
        let mut verts: Vec<Vertex> = Vec::with_capacity(scene.verts.len());
        let mut idx: Vec<u32> = Vec::with_capacity(scene.indices.len());
        for d in &scene.draws {
            if !d.active {
                continue;
            }
            let base = verts.len() as u32;
            let slot = d.slot;
            let (ax, ay, asz) = if slot < 255 {
                (
                    (slot % 16) as f32 / 16.0,
                    (slot / 16) as f32 / 16.0,
                    1.0 / 16.0,
                )
            } else {
                (0.0, 0.0, 1.0)
            };
            let s = d.idx_off as usize;
            let e = (d.idx_off + d.idx_count) as usize;
            let mut k = s;
            while k + 2 < e {
                let (ia, ib, ic) = (
                    scene.indices[k] as usize,
                    scene.indices[k + 1] as usize,
                    scene.indices[k + 2] as usize,
                );
                let (a, b, c) = (
                    d.model.transform_point3(scene.verts[ia]),
                    d.model.transform_point3(scene.verts[ib]),
                    d.model.transform_point3(scene.verts[ic]),
                );
                let n = (b - a).cross(c - a).normalize_or_zero();
                for (p, vi) in [(a, ia), (b, ib), (c, ic)] {
                    verts.push(Vertex {
                        pos: p.to_array(),
                        normal: n.to_array(),
                        uv: scene.uvs[vi],
                        atlas: [ax, ay, asz, slot as f32],
                    });
                }
                idx.extend_from_slice(&[
                    base + verts.len() as u32 - 3,
                    base + verts.len() as u32 - 2,
                    base + verts.len() as u32 - 1,
                ]);
                k += 3;
            }
        }
        renderer.upload(&verts, &idx);

        let spawn = Vec3::new(13.75, 21.03, -7.16); // PlayerStartPos from the real scene
        let mut world = World::new(self.difficulty.unwrap_or(Difficulty::Normal), spawn);
        world.notify("Day 1");
        // Recovered item spawn points are randomised per day by RandomKeyPlace /
        // RandomKofotPlace; the remake seeds a few near the house furniture.
        world.pickups.push(game::Pickup {
            pos: Vec3::new(14.4, 22.4, -8.6),
            item: game::Item::Key,
            taken: false,
        });
        world.pickups.push(game::Pickup {
            pos: Vec3::new(-3.5, 1.3, 2.0),
            item: game::Item::Hammer,
            taken: false,
        });
        world.pickups.push(game::Pickup {
            pos: Vec3::new(8.0, 15.4, -39.0),
            item: game::Item::BirdSeed,
            taken: false,
        });
        for d in [
            Vec3::new(11.0, 21.0, -12.0),
            Vec3::new(4.0, 1.0, 0.0),
            Vec3::new(-1.0, 7.0, 0.4),
        ] {
            world.doors.push(game::Door::new(d));
        }

        self.state = Some(State {
            window,
            surface,
            renderer,
            world,
            keys: Keys::default(),
            last: std::time::Instant::now(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            mouse_captured: false,
            last_cursor: (0.0, 0.0),
        });
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        let Some(st) = self.state.as_mut() else {
            return;
        };
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                st.renderer.config.width = size.width.max(1);
                st.renderer.config.height = size.height.max(1);
                st.surface
                    .configure(&st.renderer.device, &st.renderer.config);
                st.renderer.resize(st.renderer.config.clone());
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => {
                let on = state == ElementState::Pressed;
                match code {
                    KeyCode::KeyW | KeyCode::ArrowUp => st.keys.fwd = on,
                    KeyCode::KeyS | KeyCode::ArrowDown => st.keys.back = on,
                    KeyCode::KeyA | KeyCode::ArrowLeft => st.keys.left = on,
                    KeyCode::KeyD | KeyCode::ArrowRight => st.keys.right = on,
                    KeyCode::ShiftLeft => st.keys.run = on,
                    KeyCode::KeyQ => st.keys.turn_left = on,
                    KeyCode::KeyR => st.keys.turn_right = on,
                    KeyCode::ControlLeft => st.keys.crouch = on,
                    KeyCode::KeyE if on => st.world.notify("Interact"),
                    KeyCode::Escape if on => el.exit(),
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if st.mouse_captured {
                    st.mouse_dx += position.x as f32 - st.last_cursor.0;
                    st.mouse_dy += position.y as f32 - st.last_cursor.1;
                }
                st.last_cursor = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } => {
                st.mouse_captured = true;
            }
            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let dt = (now - st.last).as_secs_f32().min(0.05);
                st.last = now;

                let mut want = glam::Vec2::ZERO;
                if st.keys.fwd {
                    want.y += 1.0
                }
                if st.keys.back {
                    want.y -= 1.0
                }
                if st.keys.left {
                    want.x -= 1.0
                }
                if st.keys.right {
                    want.x += 1.0
                }
                st.world.player.crouch = st.keys.crouch;
                if st.keys.turn_left {
                    st.world.player.yaw += 2.2 * dt;
                }
                if st.keys.turn_right {
                    st.world.player.yaw -= 2.2 * dt;
                }
                st.world.player.yaw += st.mouse_dx * 0.0022;
                st.world.player.pitch =
                    (st.world.player.pitch - st.mouse_dy * 0.0022).clamp(-1.45, 1.45);
                st.mouse_dx = 0.0;
                st.mouse_dy = 0.0;
                st.world.tick(dt, want, st.keys.run);

                let frame = match st.surface.get_current_texture() {
                    Ok(f) => f,
                    Err(_) => return,
                };
                let view = frame.texture.create_view(&Default::default());
                let eye = if let Ok(v) = std::env::var("GRANNY_CAM") {
                    let p: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                    if p.len() == 3 {
                        Vec3::new(p[0], p[1], p[2])
                    } else {
                        st.world.player.pos + Vec3::new(0.0, 1.55, 0.0)
                    }
                } else {
                    st.world.player.pos + Vec3::new(0.0, 1.55, 0.0)
                };
                let aspect =
                    st.renderer.config.width as f32 / st.renderer.config.height.max(1) as f32;
                // rebuild dynamic entities each frame
                let mut b = entities::Builder::new();
                let g = &st.world.granny;
                b.figure(g.pos, 0.0, 1.62);
                for p in &st.world.pickups {
                    if p.taken {
                        continue;
                    }
                    b.box_at(
                        p.pos + Vec3::new(0.0, 0.18, 0.0),
                        Vec3::new(0.14, 0.14, 0.14),
                        0.6,
                    );
                }
                for d in &st.world.doors {
                    let open = matches!(d.state, game::DoorState::Open);
                    if !open {
                        b.box_at(
                            d.pos + Vec3::new(0.0, 1.05, 0.0),
                            Vec3::new(0.52, 1.05, 0.07),
                            0.0,
                        );
                    }
                }
                let ev: Vec<render::Vertex> = b
                    .verts
                    .iter()
                    .map(|v| render::Vertex {
                        pos: v.pos,
                        normal: v.normal,
                        uv: v.uv,
                        atlas: v.atlas,
                    })
                    .collect();
                st.renderer.upload_entities(&ev, &b.idx);
                st.renderer.render(
                    &view,
                    eye,
                    st.world.player.yaw,
                    st.world.player.pitch,
                    aspect,
                );
                frame.present();
                st.window.request_redraw();
            }
            _ => {}
        }
    }
}
