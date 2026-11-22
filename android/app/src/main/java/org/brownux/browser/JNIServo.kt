package org.brownux.browser

import android.view.Surface

class JNIServo {
    external fun nativeInit(): Long
    external fun nativeLoadUrl(servoPtr: Long, url: String)
    external fun nativeSurfaceChanged(servoPtr: Long, surface: Surface, width: Int, height: Int)
    external fun nativeSurfaceDestroyed(servoPtr: Long)

    companion object {
        init {
            System.loadLibrary("brownux")
        }
    }
}
