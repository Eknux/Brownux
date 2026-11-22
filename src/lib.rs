mod adblock;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;

use dpi::PhysicalSize;
use euclid::Scale;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jfloat, jint, jlong};
use jni::JNIEnv;
use ndk::native_window::NativeWindow;
use raw_window_handle::{DisplayHandle, HasWindowHandle};
use servo::{
    EventLoopWaker, InputEvent, MouseButton, MouseButtonAction, MouseButtonEvent, MouseMoveEvent,
    RenderingContext, Servo, ServoBuilder, WebView, WebViewBuilder, WebViewDelegate, WheelDelta,
    WheelEvent, WheelMode, WindowRenderingContext,
};
use url::Url;
use webrender_api::units::DevicePoint;

// Kaydirma ters giderse -1.0 yap
const SCROLL_SIGN: f64 = 1.0;
const TOUCH_SLOP: f32 = 16.0;

enum Cmd {
    Surface(NativeWindow, u32, u32, f32),
    Resize(u32, u32),
    Load(String),
    Touch(i32, f32, f32),
    Destroy,
    Tick,
}

static TX: Mutex<Option<Sender<Cmd>>> = Mutex::new(None);
static HAS_SURFACE: AtomicBool = AtomicBool::new(false);

fn send(cmd: Cmd) {
    if let Some(tx) = TX.lock().unwrap().as_ref() {
        let _ = tx.send(cmd);
    }
}

#[derive(Clone)]
struct Waker(Sender<Cmd>);

impl EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }
    fn wake(&self) {
        let _ = self.0.send(Cmd::Tick);
    }
}

struct Delegate {
    dirty: Rc<Cell<bool>>,
}

impl WebViewDelegate for Delegate {
    fn notify_new_frame_ready(&self, _: WebView) {
        self.dirty.set(true);
    }
}

struct State {
    webview: WebView,
    servo: Servo,
    rc: Rc<WindowRenderingContext>,
    dirty: Rc<Cell<bool>>,
    _window: NativeWindow,
}

fn create(
    window: NativeWindow,
    w: u32,
    h: u32,
    scale: f32,
    url: &str,
    tx: &Sender<Cmd>,
) -> Option<State> {
    let size = PhysicalSize::new(w, h);
    let rc = {
        let wh = window.window_handle().ok()?;
        match WindowRenderingContext::new(DisplayHandle::android(), wh, size) {
            Ok(c) => Rc::new(c),
            Err(_) => {
                log::error!("BRW: WindowRenderingContext olusmadi");
                return None;
            }
        }
    };
    let _ = rc.make_current();

    let servo = ServoBuilder::default()
        .event_loop_waker(Box::new(Waker(tx.clone())))
        .build();

    let dirty = Rc::new(Cell::new(false));
    let webview = WebViewBuilder::new(&servo, rc.clone())
        .url(Url::parse(url).ok()?)
        .hidpi_scale_factor(Scale::new(scale))
        .delegate(Rc::new(Delegate { dirty: dirty.clone() }))
        .build();
    log::info!("BRW: webview hazir, url={}, scale={}", url, scale);

    Some(State { webview, servo, rc, dirty, _window: window })
}

fn handle_touch(
    s: &State,
    action: i32,
    x: f32,
    y: f32,
    start: &mut (f32, f32),
    last: &mut (f32, f32),
    moved: &mut bool,
) {
    match action {
        0 => {
            *start = (x, y);
            *last = (x, y);
            *moved = false;
        }
        2 => {
            if !*moved {
                let (dx, dy) = (x - start.0, y - start.1);
                if dx * dx + dy * dy > TOUCH_SLOP * TOUCH_SLOP {
                    *moved = true;
                }
            }
            if *moved {
                let dx = (x - last.0) as f64 * SCROLL_SIGN;
                let dy = (y - last.1) as f64 * SCROLL_SIGN;
                *last = (x, y);
                s.webview.notify_input_event(InputEvent::Wheel(WheelEvent::new(
                    WheelDelta { x: dx, y: dy, z: 0.0, mode: WheelMode::DeltaPixel },
                    DevicePoint::new(x, y).into(),
                )));
            }
        }
        1 => {
            if !*moved {
                log::info!("BRW: tap {} {}", x, y);
                let p = DevicePoint::new(x, y);
                s.webview.notify_input_event(InputEvent::MouseMove(MouseMoveEvent::new(
                    p.into(),
                )));
                s.webview.notify_input_event(InputEvent::MouseButton(MouseButtonEvent::new(
                    MouseButtonAction::Down,
                    MouseButton::Left,
                    p.into(),
                )));
                s.webview.notify_input_event(InputEvent::MouseButton(MouseButtonEvent::new(
                    MouseButtonAction::Up,
                    MouseButton::Left,
                    p.into(),
                )));
            }
        }
        _ => {}
    }
}

fn run(rx: Receiver<Cmd>, tx: Sender<Cmd>) {
    let mut state: Option<State> = None;
    let mut url = String::from("https://servo.org");
    let mut t_start = (0f32, 0f32);
    let mut t_last = (0f32, 0f32);
    let mut t_moved = false;

    loop {
        match rx.recv_timeout(Duration::from_millis(16)) {
            Ok(Cmd::Surface(win, w, h, scale)) => {
                state = None;
                state = create(win, w, h, scale, &url, &tx);
                log::info!("BRW: state olustu = {}", state.is_some());
            }
            Ok(Cmd::Resize(w, h)) => {
                if let Some(s) = &state {
                    let size = PhysicalSize::new(w, h);
                    s.rc.resize(size);
                    s.webview.resize(size);
                }
            }
            Ok(Cmd::Load(u)) => {
                log::info!("BRW: load {}", u);
                url = u.clone();
                if let Some(s) = &state {
                    if let Ok(parsed) = Url::parse(&u) {
                        s.webview.load(parsed);
                    }
                }
            }
            Ok(Cmd::Touch(action, x, y)) => {
                if let Some(s) = &state {
                    handle_touch(s, action, x, y, &mut t_start, &mut t_last, &mut t_moved);
                }
            }
            Ok(Cmd::Destroy) => {
                log::info!("BRW: destroy");
                state = None;
            }
            Ok(Cmd::Tick) => {}
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if let Some(s) = &state {
            s.servo.spin_event_loop();
            if s.dirty.replace(false) {
                s.webview.paint();
                s.rc.present();
            }
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_org_brownux_browser_JNIServo_nativeInit(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    std::panic::set_hook(Box::new(|info| log::error!("BRW PANIC: {}", info)));

    let mut guard = TX.lock().unwrap();
    if guard.is_none() {
        let (tx, rx) = channel();
        let tx2 = tx.clone();
        std::thread::Builder::new()
            .name("servo".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || run(rx, tx2))
            .expect("servo thread");
        *guard = Some(tx);
    }
    1
}

#[no_mangle]
pub extern "system" fn Java_org_brownux_browser_JNIServo_nativeLoadUrl(
    mut env: JNIEnv,
    _class: JClass,
    _servo_ptr: jlong,
    url: JString,
) {
    let url_str: String = env.get_string(&url).unwrap().into();

    if adblock::should_block(&url_str, "document") {
        log::warn!("BRW: ana dokuman engellendi: {}", url_str);
        return;
    }
    send(Cmd::Load(url_str));
}

#[no_mangle]
pub extern "system" fn Java_org_brownux_browser_JNIServo_nativeTouch(
    _env: JNIEnv,
    _class: JClass,
    _servo_ptr: jlong,
    action: jint,
    x: jfloat,
    y: jfloat,
) {
    send(Cmd::Touch(action, x, y));
}

#[no_mangle]
pub extern "system" fn Java_org_brownux_browser_JNIServo_nativeSurfaceChanged(
    env: JNIEnv,
    _class: JClass,
    _servo_ptr: jlong,
    surface: JObject,
    width: jint,
    height: jint,
    density: jfloat,
) {
    let (w, h) = (width.max(1) as u32, height.max(1) as u32);

    if HAS_SURFACE.load(Ordering::SeqCst) {
        send(Cmd::Resize(w, h));
        return;
    }

    let window = unsafe {
        NativeWindow::from_surface(env.get_raw() as *mut _, surface.as_raw() as *mut _)
    };
    if let Some(window) = window {
        HAS_SURFACE.store(true, Ordering::SeqCst);
        send(Cmd::Surface(window, w, h, density));
    } else {
        log::error!("BRW: Surface -> ANativeWindow basarisiz");
    }
}

#[no_mangle]
pub extern "system" fn Java_org_brownux_browser_JNIServo_nativeSurfaceDestroyed(
    _env: JNIEnv,
    _class: JClass,
    _servo_ptr: jlong,
) {
    HAS_SURFACE.store(false, Ordering::SeqCst);
    send(Cmd::Destroy);
}
