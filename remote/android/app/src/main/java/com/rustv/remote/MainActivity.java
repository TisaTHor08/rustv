package com.rustv.remote;

import android.os.Bundle;
import android.view.KeyEvent;
import android.webkit.JavascriptInterface;

import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    /** Vrai quand l'appli est connectée à un PC : les boutons de volume du téléphone règlent alors le volume du PC. */
    private volatile boolean captureVolume = false;

    private class NativeBridge {
        @JavascriptInterface
        public void setVolumeCapture(boolean on) {
            captureVolume = on;
        }
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        registerPlugin(RusTVTransportPlugin.class); // avant super.onCreate
        super.onCreate(savedInstanceState);
        getBridge().getWebView().addJavascriptInterface(new NativeBridge(), "RusTVNative");
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        int code = event.getKeyCode();
        if (captureVolume && (code == KeyEvent.KEYCODE_VOLUME_UP || code == KeyEvent.KEYCODE_VOLUME_DOWN)) {
            // Appui initial et répétitions (maintien) : on transmet à l'interface web, sans toucher au volume du téléphone.
            if (event.getAction() == KeyEvent.ACTION_DOWN) {
                final String dir = code == KeyEvent.KEYCODE_VOLUME_UP ? "up" : "down";
                getBridge().getWebView().post(() ->
                    getBridge().getWebView().evaluateJavascript(
                        "window.dispatchEvent(new CustomEvent('rustv-volkey',{detail:'" + dir + "'}))", null));
            }
            return true;
        }
        return super.dispatchKeyEvent(event);
    }
}
