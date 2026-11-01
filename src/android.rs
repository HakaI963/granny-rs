//! Android entry point: `NativeActivity` + wgpu surface.
//!
//! `android_main` runs before a window exists. `MainEvent::InitWindow` is the
//! first moment `native_window()` is valid, so the wgpu surface and the game are
//! created there and torn down on `TerminateWindow`.

#![cfg(target_os = "android")]

use crate::app::App;
use crate::game::Difficulty;
use android_activity::input::{InputEvent, KeyAction, Keycode, MotionAction};
use android_activity::{AndroidApp, MainEvent, PollEvent};
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, DisplayHandle, HandleError, HasDisplayHandle,
    HasWindowHandle, WindowHandle,
};

struct AndroidWindow {
    window: *mut ndk_sys::ANativeWindow,
}

// SAFETY: the ANativeWindow is owned by the activity and only used from the
// thread that drives `android_main`.
unsafe impl Send for AndroidWindow {}
unsafe impl Sync for AndroidWindow {}

impl HasWindowHandle for AndroidWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = AndroidNdkWindowHandle::new(
            std::ptr::NonNull::new(self.window.cast::<std::ffi::c_void>()).unwrap(),
        );
        Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
    }
}

impl HasDisplayHandle for AndroidWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let handle = AndroidDisplayHandle::new();
        Ok(unsafe { DisplayHandle::borrow_raw(handle.into()) })
    }
}

struct Renderer {
    surface: wgpu::Surface<'static>,
    game: App,
}

/// Build the surface and the game once a window exists.
fn init_renderer(app: &AndroidApp) -> anyhow::Result<Renderer> {
    let native = app
        .native_window()
        .ok_or_else(|| anyhow::anyhow!("no native window"))?;
    let (width, height) = (native.width().max(1) as u32, native.height().max(1) as u32);
    let raw = native.ptr().as_ptr().cast::<ndk_sys::ANativeWindow>();
    // The surface borrows the window for the process lifetime; leak the handle
    // so the surface can be 'static, which wgpu requires.
    let target: &'static AndroidWindow = Box::leak(Box::new(AndroidWindow { window: raw }));

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let surface = instance.create_surface(target)?;
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .ok_or_else(|| anyhow::anyhow!("no vulkan adapter"))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;

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
        width,
        height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let game = App::new(device, queue, config, Difficulty::Normal)?;
    Ok(Renderer { surface, game })
}

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    log_panics::init();

    crate::assets::set_android_assets(app.asset_manager());

    let mut state: Option<Renderer> = None;

    loop {
        let mut exit = false;
        app.poll_events(
            Some(std::time::Duration::from_millis(16)),
            |event| match event {
                PollEvent::Main(MainEvent::InitWindow { .. }) => {
                    if state.is_none() {
                        match init_renderer(&app) {
                            Ok(r) => {
                                log::info!("renderer ready");
                                state = Some(r);
                            }
                            Err(e) => {
                                log::error!("init failed: {e}");
                                exit = true;
                            }
                        }
                    }
                }
                PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                    state = None;
                }
                PollEvent::Main(MainEvent::ConfigChanged { .. })
                | PollEvent::Main(MainEvent::ContentRectChanged { .. })
                | PollEvent::Main(MainEvent::WindowResized { .. }) => {
                    if let Some(r) = state.as_mut() {
                        // The window includes system insets; the content rect is
                        // what is actually visible and what we must render into.
                        let rect = app.content_rect();
                        let (w, h) = (
                            ((rect.right - rect.left) as u32).max(1),
                            ((rect.bottom - rect.top) as u32).max(1),
                        );
                        r.game.renderer.config.width = w;
                        r.game.renderer.config.height = h;
                        r.surface
                            .configure(&r.game.renderer.device, &r.game.renderer.config);
                        r.game.renderer.resize(r.game.renderer.config.clone());
                    }
                }
                PollEvent::Main(MainEvent::RedrawNeeded { .. }) => {
                    if let Some(r) = state.as_mut() {
                        if let Ok(frame) = r.surface.get_current_texture() {
                            let view = frame.texture.create_view(&Default::default());
                            let (w, h) =
                                (r.game.renderer.config.width, r.game.renderer.config.height);
                            r.game.frame(&view, w, h);
                            frame.present();
                        }
                    }
                }
                PollEvent::Main(MainEvent::InputAvailable) => {
                    if let Some(r) = state.as_mut() {
                        if let Ok(mut iter) = app.input_events_iter() {
                            while iter.next(|e| {
                                apply_input(&mut r.game, e);
                                android_activity::InputStatus::Handled
                            }) {}
                        }
                    }
                }
                PollEvent::Main(MainEvent::Destroy { .. }) => exit = true,
                _ => {}
            },
        );
        if exit {
            return;
        }
    }
}

fn apply_input(game: &mut App, e: &InputEvent) {
    match e {
        InputEvent::KeyEvent(k) => {
            let on = k.action() == KeyAction::Down;
            match k.key_code() {
                Keycode::W => game.keys.fwd = on,
                Keycode::S => game.keys.back = on,
                Keycode::A => game.keys.left = on,
                Keycode::D => game.keys.right = on,
                Keycode::Q => game.keys.turn_left = on,
                Keycode::R => game.keys.turn_right = on,
                _ => {}
            }
        }
        InputEvent::MotionEvent(m) => {
            if m.action() == MotionAction::Move && m.pointer_count() > 0 {
                let p = m.pointer_at_index(0);
                game.mouse_dx += p.axis_value(android_activity::input::Axis::X) * 0.35;
                game.mouse_dy += p.axis_value(android_activity::input::Axis::Y) * 0.35;
            }
        }
        _ => {}
    }
}
