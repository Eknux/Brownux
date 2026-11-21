mod adblock;

use jni::objects::{JClass, JString};
use jni::sys::jlong;
use jni::JNIEnv;
use servo::WebViewBuilder;
use servo::Servo;
use std::sync::Mutex;
use std::sync::Arc;

static SERVO: once_cell::sync::Lazy<Mutex<Option<Arc<Servo>>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

#[no_mangle]
pub extern "system" fn Java_org_brownux_browser_JNIServo_nativeInit(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let servo = Arc::new(servo::ServoBuilder::default().build());
    let mut lock = SERVO.lock().unwrap();
    *lock = Some(servo.clone());

    Arc::into_raw(servo) as jlong
}

#[no_mangle]
pub extern "system" fn Java_org_brownux_browser_JNIServo_nativeLoadUrl(
    mut env: JNIEnv,
    _class: JClass,
    servo_ptr: jlong,
    url: JString,
) {
    let url_str: String = env.get_string(&url).unwrap().into();

    if adblock::should_block(&url_str, "document") {
        log::warn!("Brownux ana dokümanı engelledi: {}", url_str);
        return;
    }

    if servo_ptr == 0 {
        log::error!("Servo işaretçisi null");
        return;
    }

    let servo: &Servo = unsafe { &*(servo_ptr as *const Servo) };
    let _webview = WebViewBuilder::new(servo)
        .url(url_str.parse().expect("Geçersiz URL"))
        .build();

    log::info!("Brownux yüklüyor: {}", url_str);
}
