package com.rustv.remote;

import android.Manifest;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothManager;
import android.bluetooth.BluetoothSocket;
import android.content.Context;
import android.net.wifi.WifiManager;
import android.os.Build;
import android.util.Base64;

import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;
import com.getcapacitor.annotation.Permission;
import com.getcapacitor.annotation.PermissionCallback;

import java.io.Closeable;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * Transport brut pour la WebView : flux TCP, flux Bluetooth RFCOMM et écoute du beacon UDP.
 * Aucune logique applicative ici : les octets sont relayés en base64 vers le TypeScript,
 * qui implémente le protocole (trames, handshake, chiffrement).
 */
@CapacitorPlugin(
    name = "RusTVTransport",
    permissions = {
        @Permission(alias = "bluetooth", strings = { Manifest.permission.BLUETOOTH_CONNECT })
    }
)
public class RusTVTransportPlugin extends Plugin {

    private static final class Conn {
        final OutputStream out;
        final Closeable closer;
        Conn(OutputStream out, Closeable closer) {
            this.out = out;
            this.closer = closer;
        }
    }

    private final AtomicInteger nextId = new AtomicInteger(1);
    private final ConcurrentHashMap<Integer, Conn> conns = new ConcurrentHashMap<>();
    private DatagramSocket beacon;
    private WifiManager.MulticastLock multicastLock;

    // ---------- connexions ----------

    private int register(InputStream in, OutputStream out, Closeable closer) {
        final int id = nextId.getAndIncrement();
        conns.put(id, new Conn(out, closer));
        new Thread(() -> {
            byte[] buf = new byte[8192];
            try {
                int n;
                while ((n = in.read(buf)) > 0) {
                    JSObject ev = new JSObject();
                    ev.put("id", id);
                    ev.put("data", Base64.encodeToString(buf, 0, n, Base64.NO_WRAP));
                    notifyListeners("data", ev);
                }
            } catch (Exception ignored) {
            }
            closeConn(id);
        }, "rustv-read-" + id).start();
        return id;
    }

    private void closeConn(int id) {
        Conn c = conns.remove(id);
        if (c == null) return; // déjà fermée : un seul événement « closed »
        try { c.closer.close(); } catch (Exception ignored) { }
        JSObject ev = new JSObject();
        ev.put("id", id);
        notifyListeners("closed", ev);
    }

    @PluginMethod
    public void tcpConnect(PluginCall call) {
        final String host = call.getString("host");
        final int port = call.getInt("port", 47820);
        final int timeout = call.getInt("timeoutMs", 4000);
        new Thread(() -> {
            try {
                Socket s = new Socket();
                s.setTcpNoDelay(true);
                s.connect(new InetSocketAddress(host, port), timeout);
                int id = register(s.getInputStream(), s.getOutputStream(), s);
                JSObject r = new JSObject();
                r.put("id", id);
                call.resolve(r);
            } catch (Exception e) {
                call.reject("tcp: " + e.getMessage());
            }
        }, "rustv-tcp-connect").start();
    }

    @PluginMethod
    public void btConnect(PluginCall call) {
        if (Build.VERSION.SDK_INT >= 31 && getPermissionState("bluetooth") != com.getcapacitor.PermissionState.GRANTED) {
            requestPermissionForAlias("bluetooth", call, "btPermsCallback");
            return;
        }
        doBtConnect(call);
    }

    @PermissionCallback
    private void btPermsCallback(PluginCall call) {
        if (getPermissionState("bluetooth") == com.getcapacitor.PermissionState.GRANTED) {
            doBtConnect(call);
        } else {
            call.reject("permission Bluetooth refusée");
        }
    }

    private void doBtConnect(PluginCall call) {
        final String address = call.getString("address");
        final String uuid = call.getString("uuid");
        new Thread(() -> {
            BluetoothSocket sock = null;
            try {
                BluetoothManager mgr = (BluetoothManager) getContext().getSystemService(Context.BLUETOOTH_SERVICE);
                BluetoothAdapter adapter = mgr.getAdapter();
                if (adapter == null || !adapter.isEnabled()) throw new Exception("Bluetooth désactivé");
                BluetoothDevice dev = adapter.getRemoteDevice(address);
                sock = dev.createRfcommSocketToServiceRecord(UUID.fromString(uuid));
                adapter.cancelDiscovery();
                sock.connect();
                int id = register(sock.getInputStream(), sock.getOutputStream(), sock);
                JSObject r = new JSObject();
                r.put("id", id);
                call.resolve(r);
            } catch (Exception e) {
                try { if (sock != null) sock.close(); } catch (Exception ignored) { }
                call.reject("bt: " + e.getMessage());
            }
        }, "rustv-bt-connect").start();
    }

    @PluginMethod
    public void write(PluginCall call) {
        Conn c = conns.get(call.getInt("id", -1));
        if (c == null) {
            call.reject("connexion inconnue");
            return;
        }
        try {
            byte[] data = Base64.decode(call.getString("data", ""), Base64.NO_WRAP);
            synchronized (c) {
                c.out.write(data);
                c.out.flush();
            }
            call.resolve();
        } catch (Exception e) {
            closeConn(call.getInt("id", -1));
            call.reject("write: " + e.getMessage());
        }
    }

    @PluginMethod
    public void close(PluginCall call) {
        closeConn(call.getInt("id", -1));
        call.resolve();
    }

    // ---------- beacon UDP ----------

    @PluginMethod
    public synchronized void startBeacon(PluginCall call) {
        final int port = call.getInt("port", 47821);
        stopBeaconInternal();
        try {
            WifiManager wifi = (WifiManager) getContext().getApplicationContext().getSystemService(Context.WIFI_SERVICE);
            if (wifi != null) {
                multicastLock = wifi.createMulticastLock("rustv");
                multicastLock.setReferenceCounted(false);
                multicastLock.acquire(); // certains téléphones filtrent le broadcast sans ce verrou
            }
            final DatagramSocket s = new DatagramSocket(null);
            s.setReuseAddress(true);
            s.bind(new InetSocketAddress(port));
            beacon = s;
            new Thread(() -> {
                byte[] buf = new byte[2048];
                while (!s.isClosed()) {
                    try {
                        DatagramPacket p = new DatagramPacket(buf, buf.length);
                        s.receive(p);
                        JSObject ev = new JSObject();
                        ev.put("ip", p.getAddress().getHostAddress());
                        ev.put("payload", new String(p.getData(), 0, p.getLength(), StandardCharsets.UTF_8));
                        notifyListeners("beacon", ev);
                    } catch (Exception e) {
                        break;
                    }
                }
            }, "rustv-beacon").start();
            call.resolve();
        } catch (Exception e) {
            call.reject("beacon: " + e.getMessage());
        }
    }

    @PluginMethod
    public synchronized void stopBeacon(PluginCall call) {
        stopBeaconInternal();
        call.resolve();
    }

    private void stopBeaconInternal() {
        if (beacon != null) { beacon.close(); beacon = null; }
        if (multicastLock != null && multicastLock.isHeld()) multicastLock.release();
    }

    @Override
    protected void handleOnDestroy() {
        stopBeaconInternal();
        for (Integer id : conns.keySet()) closeConn(id);
    }
}
