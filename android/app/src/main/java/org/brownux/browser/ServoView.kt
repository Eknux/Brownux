package org.brownux.browser

import android.content.Context
import android.util.AttributeSet
import android.view.SurfaceHolder
import android.view.SurfaceView

class ServoView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null
) : SurfaceView(context, attrs), SurfaceHolder.Callback {

    private val jni = JNIServo()
    private var servoPtr: Long = 0

    init {
        holder.addCallback(this)
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        if (servoPtr == 0L) servoPtr = jni.nativeInit()
        loadUrl("https://servo.org")
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (servoPtr != 0L) jni.nativeSurfaceChanged(servoPtr, holder.surface, width, height)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        if (servoPtr != 0L) jni.nativeSurfaceDestroyed(servoPtr)
    }

    fun loadUrl(url: String) {
        if (servoPtr != 0L) jni.nativeLoadUrl(servoPtr, url)
    }
}
