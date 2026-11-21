package org.brownux.browser

class JNIServo {
    external fun nativeInit(): Long
    external fun nativeLoadUrl(servoPtr: Long, url: String)

    companion object {
        init {
            System.loadLibrary("brownux")
        }
    }
}