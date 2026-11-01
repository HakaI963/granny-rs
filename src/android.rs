//! Android entry point: `NativeActivity` + wgpu surface.

#![cfg(target_os = "android")]

use android_activity::input::{InputEvent, KeyAction, Keycode, MotionAction};
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use granny_rs::app::App;
use granny_rs::game::Difficulty;
use std::sync::Arc;

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    log_panics::init();

    if let Some(mgr) = app.asset_manager() {
        granny_rs::assets::set_android_assets(mgr);
    }

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let surface = instance
        .create_surface(unsafe { wgpu::SurfaceTargetUnsafe::from_window(&app).unwrap() })
        .expect("surface");
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
    let size = app
        .native_window()
        .map(|w| (w.width() as u32, w.height() as u32))
        .unwrap_or((1280, 720));
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

    let mut app = app;
    loop {
        app.poll_events(
            Some(std::time::Duration::from_millis(16)),
            |event| match event {
                PollEvent::Main(MainEvent::Resized { width, height }) => {
                    game.renderer.config.width = width.max(1);
                    game.renderer.config.height = height.max(1);
                    surface.configure(&game.renderer.device, &game.renderer.config);
                    game.renderer.resize(game.renderer.config.clone());
                }
                PollEvent::Main(MainEvent::RedrawNeeded) => {
                    if let Ok(frame) = surface.get_current_texture() {
                        let view = frame.texture.create_view(&Default::default());
                        let (w, h) = (game.renderer.config.width, game.renderer.config.height);
                        game.frame(&view, w, h);
                        frame.present();
                    }
                }
                PollEvent::Main(MainEvent::InputAvailable) => {
                    let events: Vec<_> = app.input_events_iter().flatten().collect();
                    for e in events {
                        apply_input(&mut game, e);
                    }
                }
                PollEvent::Main(MainEvent::Destroy) => std::process::exit(0),
                _ => {}
            },
        );
    }
}

fn apply_input(game: &mut App, e: InputEvent) {
    match e {
        InputEvent::KeyEvent(k) => {
            let on = k.key_action == KeyAction::Down;
            match k.key_code {
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
            if m.action() == MotionAction::Move {
                game.mouse_dx += m.axis_value(android_activity::input::MotionAxis::X) * 0.35;
                game.mouse_dy += m.axis_value(android_activity::input::MotionAxis::Y) * 0.35;
            }
        }
        _ => {}
    }
}
