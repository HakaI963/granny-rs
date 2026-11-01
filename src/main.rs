//! Desktop entry point (winit event loop).

use granny_rs::app::App;
use granny_rs::game::Difficulty;
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
    let mut app = Desktop::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct Desktop {
    state: Option<State>,
    difficulty: Option<Difficulty>,
}

struct State {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    app: App,
    mouse_captured: bool,
    last_cursor: (f32, f32),
}

impl ApplicationHandler for Desktop {
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

        let app = App::new(
            device,
            queue,
            config,
            self.difficulty.unwrap_or(Difficulty::Normal),
        )
        .expect("init app");

        self.state = Some(State {
            window,
            surface,
            app,
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
                st.app.renderer.config.width = size.width.max(1);
                st.app.renderer.config.height = size.height.max(1);
                st.surface
                    .configure(&st.app.renderer.device, &st.app.renderer.config);
                st.app.renderer.resize(st.app.renderer.config.clone());
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
                let k = &mut st.app.keys;
                match code {
                    KeyCode::KeyW | KeyCode::ArrowUp => k.fwd = on,
                    KeyCode::KeyS | KeyCode::ArrowDown => k.back = on,
                    KeyCode::KeyA | KeyCode::ArrowLeft => k.left = on,
                    KeyCode::KeyD | KeyCode::ArrowRight => k.right = on,
                    KeyCode::KeyQ => k.turn_left = on,
                    KeyCode::KeyR => k.turn_right = on,
                    KeyCode::ShiftLeft => k.run = on,
                    KeyCode::ControlLeft => k.crouch = on,
                    KeyCode::KeyE if on => st.app.world.notify("Interact"),
                    KeyCode::Escape if on => el.exit(),
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if st.mouse_captured {
                    st.app.mouse_dx += position.x as f32 - st.last_cursor.0;
                    st.app.mouse_dy += position.y as f32 - st.last_cursor.1;
                }
                st.last_cursor = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } => st.mouse_captured = true,
            WindowEvent::RedrawRequested => {
                let frame = match st.surface.get_current_texture() {
                    Ok(f) => f,
                    Err(_) => return,
                };
                let view = frame.texture.create_view(&Default::default());
                let (w, h) = (st.app.renderer.config.width, st.app.renderer.config.height);
                st.app.frame(&view, w, h);
                frame.present();
                st.window.request_redraw();
            }
            _ => {}
        }
    }
}
