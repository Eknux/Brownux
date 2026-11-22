mod adblock;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;

use dpi::PhysicalSize;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jint, jlong};
use jni::JNIEnv;
use ndk::native_window::NativeWindow;
use raw_window_handle::{DisplayHandle, HasWindowHandle};
use servo::{
    EventLoopWaker, RenderingContext, Servo, ServoBuilder, WebView, WebViewBuilder,
    WebViewDelegate, WindowRenderingContext,
};
use url::Url;

enum Cmd {
    Surface(NativeWindow, u32, u32),
    Resize(u32, u32),
    Load(String),
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
        log::info!("BRW: frame ready");
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

fn create(window: NativeWindow, w: u32, h: u32, url: &str, tx: &Sender<Cmd>) -> Option<State> {
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
    log::info!("BRW: rendering context hazir");

    let servo = ServoBuilder::default()
        .event_loop_waker(Box::new(Waker(tx.clone())))
        .build();
    log::info!("BRW: servo hazir");

    let dirty = Rc::new(Cell::new(false));
    let webview = WebViewBuilder::new(&servo, rc.clone())
        .url(Url::parse(url).ok()?)
        .delegate(Rc::new(Delegate { dirty: dirty.clone() }))
        .build();
    log::info!("BRW: webview hazir, url={}", url);

    Some(State { webview, servo, rc, dirty, _window: window })
}

fn run(rx: Receiver<Cmd>, tx: Sender<Cmd>) {
    let mut state: Option<State> = None;
    let mut url = String::from("https://servo.org");
    let mut ticks: u64 = 0;

    loop {
        match rx.recv_timeout(Duration::from_millis(16)) {
            Ok(Cmd::Surface(win, w, h)) => {
                log::info!("BRW: surface {}x{}", w, h);
                state = None;
                state = create(win, w, h, &url, &tx);
                log::info!("BRW: state olustu = {}", state.is_some());
            }
            Ok(Cmd::Resize(w, h)) => {
                log::info!("BRW: resize {}x{}", w, h);
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
            Ok(Cmd::Destroy) => {
                log::info!("BRW: destroy");
                state = None;
            }
            Ok(Cmd::Tick) => {}
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        ticks += 1;
        if ticks % 120 == 0 {
            log::info!("BRW: dongu canli, tick {}", ticks);
        }

        if let Some(s) = &state {
            s.servo.spin_event_loop();
            if s.dirty.replace(false) {
                s.webview.paint();
                s.rc.present();
                log::info!("BRW: paint+present");
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
pub extern "system" fn Java_org_brownux_browser_JNIServo_nativeSurfaceChanged(
    env: JNIEnv,
    _class: JClass,
    _servo_ptr: jlong,
    surface: JObject,
    width: jint,
    height: jint,
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
        send(Cmd::Surface(window, w, h));
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
