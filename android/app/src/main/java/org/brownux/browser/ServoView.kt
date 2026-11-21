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
        servoPtr = jni.nativeInit()
        loadUrl("https://servo.org") // Test için başlangıç sayfası
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        // Yüzey boyutu değişiklikleri
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        // Temizlik işlemleri
    }

    fun loadUrl(url: String) {
        if (servoPtr != 0L) {
            jni.nativeLoadUrl(servoPtr, url)
        }
    }
}