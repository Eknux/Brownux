package org.brownux.browser

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // Material You kenardan kenara görünüm
        WindowCompat.setDecorFitsSystemWindows(window, false)
        
        val servoView = ServoView(this)
        setContentView(servoView)
    }
}