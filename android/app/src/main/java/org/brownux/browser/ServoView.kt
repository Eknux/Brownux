package org.brownux.browser

import android.content.Context
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.inputmethod.InputMethodManager

class ServoView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null
) : SurfaceView(context, attrs), SurfaceHolder.Callback {

    private val jni = JNIServo()
    private var servoPtr: Long = 0

    init {
        holder.addCallback(this)
        isFocusableInTouchMode = true
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        if (servoPtr == 0L) servoPtr = jni.nativeInit()
        loadUrl("https://servo.org")
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (servoPtr != 0L) {
            jni.nativeSurfaceChanged(
                servoPtr, holder.surface, width, height,
                resources.displayMetrics.density
            )
        }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        if (servoPtr != 0L) jni.nativeSurfaceDestroyed(servoPtr)
    }

    override fun onTouchEvent(e: MotionEvent): Boolean {
        if (servoPtr == 0L) return true
        if (e.actionMasked == MotionEvent.ACTION_DOWN) {
            requestFocus()
            val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
            imm.hideSoftInputFromWindow(windowToken, 0)
        }
        jni.nativeTouch(servoPtr, e.actionMasked, e.x, e.y)
        return true
    }

    fun loadUrl(url: String) {
        if (servoPtr != 0L) jni.nativeLoadUrl(servoPtr, url)
    }
}
