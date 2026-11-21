mod adblock;

use jni::objects::{JClass, JString};
use jni::sys::jlong;
use jni::JNIEnv;
use servo::Servo;
use std::sync::Arc;

#[no_mangle]
pub extern "system" fn Java_org_brownux_browser_JNIServo_nativeInit(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let servo = Arc::new(servo::ServoBuilder::default().build());
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

    // GEÇİCİ: WebView henüz yok, ekran (Surface) bağlanınca eklenecek
    log::info!("Brownux yüklenecek: {}", url_str);
}
