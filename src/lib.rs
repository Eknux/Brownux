mod adblock;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

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
        Rc::new(WindowRenderingContext::new(DisplayHandle::android(), wh, size).ok()?)
    };
    let _ = rc.make_current();

    let servo = ServoBuilder::default()
        .event_loop_waker(Box::new(Waker(tx.clone())))
        .build();

    let dirty = Rc::new(Cell::new(false));
    let webview = WebViewBuilder::new(&servo, rc.clone())
        .url(Url::parse(url).ok()?)
        .delegate(Rc::new(Delegate { dirty: dirty.clone() }))
        .build();

    Some(State { webview, servo, rc, dirty, _window: window })
}

fn run(rx: Receiver<Cmd>, tx: Sender<Cmd>) {
    let mut state: Option<State> = None;
    let mut url = String::from("https://servo.org");

    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Surface(win, w, h) => {
                state = None;
                state = create(win, w, h, &url, &tx);
                if state.is_none() {
                    log::error!("Brownux: Servo olusturulamadi");
                }
            }
            Cmd::Resize(w, h) => {
                if let Some(s) = &state {
                    let size = PhysicalSize::new(w, h);
                    s.rc.resize(size);
                    s.webview.resize(size);
                }
            }
            Cmd::Load(u) => {
                url = u.clone();
                if let Some(s) = &state {
                    if let Ok(parsed) = Url::parse(&u) {
                        s.webview.load(parsed);
                    }
                }
            }
            Cmd::Destroy => state = None,
            Cmd::Tick => {}
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
        log::warn!("Brownux ana dokumani engelledi: {}", url_str);
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
        log::error!("Brownux: Surface -> ANativeWindow basarisiz");
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
