//! Android entry point: `NativeActivity` + wgpu surface.
//!
//! wgpu needs a `HasWindowHandle`/`HasDisplayHandle` pair. `AndroidApp` exposes
//! a raw `ANativeWindow`, so this module wraps it in a tiny handle type instead
//! of relying on an integration crate that may not match our wgpu version.

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

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    log_panics::init();

    let mgr = app.asset_manager();
    crate::assets::set_android_assets(mgr);

    let raw = app
        .native_window()
        .expect("native window")
        .ptr()
        .as_ptr()
        .cast::<ndk_sys::ANativeWindow>();
    let target = AndroidWindow { window: raw };

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let surface = instance.create_surface(&target).expect("create surface");
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .expect("adapter");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
            .unwrap();

    let caps = surface.get_capabilities(&adapter);
    let format = caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(caps.formats[0]);
    let size = {
        let w = app.native_window().unwrap();
        (w.width() as u32, w.height() as u32)
    };
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.0.max(1),
        height: size.1.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let mut game = App::new(device, queue, config, Difficulty::Normal).expect("init");

    loop {
        app.poll_events(
            Some(std::time::Duration::from_millis(16)),
            |event| match event {
                PollEvent::Main(MainEvent::WindowResized { .. }) => {
                    if let Some(w) = app.native_window() {
                        let (width, height) = (w.width() as u32, w.height() as u32);
                        game.renderer.config.width = width.max(1);
                        game.renderer.config.height = height.max(1);
                        surface.configure(&game.renderer.device, &game.renderer.config);
                        game.renderer.resize(game.renderer.config.clone());
                    }
                }
                PollEvent::Main(MainEvent::RedrawNeeded { .. }) => {
                    if let Ok(frame) = surface.get_current_texture() {
                        let view = frame.texture.create_view(&Default::default());
                        let (w, h) = (game.renderer.config.width, game.renderer.config.height);
                        game.frame(&view, w, h);
                        frame.present();
                    }
                }
                PollEvent::Main(MainEvent::InputAvailable) => {
                    if let Ok(mut iter) = app.input_events_iter() {
                        while iter.next(|e| {
                            apply_input(&mut game, e);
                            android_activity::InputStatus::Handled
                        }) {}
                    }
                }
                PollEvent::Main(MainEvent::Destroy { .. }) => std::process::exit(0),
                _ => {}
            },
        );
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
