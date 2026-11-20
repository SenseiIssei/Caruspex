// The ways to the adapter that only Android's Java side can open: classic
// Bluetooth (the serial port profile of most ELM327 dongles), Bluetooth LE
// adapters over GATT and USB serial adapters on an OTG cable. The Rust side
// (src-tauri/src/android.rs) turns an open link into a
// caruspex_link::Transport: write, read with a timeout, flush, close, the
// bytes travel as base64. For Bluetooth LE the Rust side also decides which
// characteristics to use and whether the device is an OBD adapter at all;
// nothing is written to a device before it said so (bleSetup). Also here: the runtime
// permissions for Bluetooth, binding the process to the WLAN of a WLAN
// adapter, and reading and writing the files the system file dialog hands
// out (content:// addresses).
package dev.senseiissei.caruspex

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.app.PendingIntent
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.BluetoothSocket
import android.bluetooth.BluetoothStatusCodes
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbManager
import android.location.LocationManager
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.Uri
import android.os.Build
import android.provider.OpenableColumns
import android.util.Base64
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.location.LocationManagerCompat
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.hoho.android.usbserial.driver.UsbSerialPort
import com.hoho.android.usbserial.driver.UsbSerialProber
import com.hoho.android.usbserial.util.SerialInputOutputManager
import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.io.OutputStream
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

@InvokeArg
class LinkArgs {
  var id: Int = 0
}

@InvokeArg
class WriteArgs {
  var id: Int = 0
  var data: String = ""
}

@InvokeArg
class ReadArgs {
  var id: Int = 0
  var timeout: Long = 0
}

@InvokeArg
class BluetoothArgs {
  var address: String = ""
}

@InvokeArg
class BleScanArgs {
  var timeout: Long = 8000
}

@InvokeArg
class BleConnectArgs {
  var address: String = ""
  var timeout: Long = 15000
}

@InvokeArg
class BleSetupArgs {
  var id: Int = 0
  var service: Int = 0
  var notify: Int = 0
  var write: Int = 0
  var noResponse: Boolean = false
  var indicate: Boolean = false
  var mtu: Int = 517
}

@InvokeArg
class UsbArgs {
  var device: String = ""
  var baud: Int = 38400
}

@InvokeArg
class WifiArgs {
  var on: Boolean = false
}

@InvokeArg
class UriArgs {
  var uri: String = ""
  var data: String = ""
}

/** Received bytes, waited for with a timeout. */
private class Inbox {
  private val lock = Object()
  private val buffer = ByteArrayOutputStream()
  @Volatile var closed = false

  fun put(bytes: ByteArray, count: Int) {
    synchronized(lock) {
      buffer.write(bytes, 0, count)
      lock.notifyAll()
    }
  }

  /** Whatever is there, or the first bytes within the timeout. */
  fun take(timeoutMs: Long): ByteArray {
    val deadline = System.currentTimeMillis() + timeoutMs.coerceAtLeast(1)
    synchronized(lock) {
      while (buffer.size() == 0 && !closed) {
        val left = deadline - System.currentTimeMillis()
        if (left <= 0) break
        lock.wait(left)
      }
      val out = buffer.toByteArray()
      buffer.reset()
      return out
    }
  }

  fun clear() {
    synchronized(lock) { buffer.reset() }
  }

  fun close() {
    synchronized(lock) {
      closed = true
      lock.notifyAll()
    }
  }
}

private abstract class Link {
  val inbox = Inbox()
  abstract fun write(data: ByteArray): Boolean
  abstract fun close()
}

/** A classic Bluetooth serial port; a thread reads into the inbox. */
private class BluetoothLink(private val socket: BluetoothSocket) : Link() {
  private val output: OutputStream = socket.outputStream
  private val input: InputStream = socket.inputStream
  private val reader = Thread {
    val chunk = ByteArray(512)
    try {
      while (true) {
        val n = input.read(chunk)
        if (n < 0) break
        if (n > 0) inbox.put(chunk, n)
      }
    } catch (_: Exception) {
      // The socket was closed.
    }
    inbox.close()
  }

  init {
    reader.isDaemon = true
    reader.name = "caruspex-bt-read"
    reader.start()
  }

  override fun write(data: ByteArray): Boolean =
    try {
      output.write(data)
      output.flush()
      true
    } catch (_: Exception) {
      false
    }

  override fun close() {
    try { socket.close() } catch (_: Exception) {}
    inbox.close()
  }
}

/**
 * A Bluetooth LE adapter over GATT. connect() only connects and reads the
 * services; writing is possible after setup(), which the Rust side calls
 * only for a device it recognised as an OBD adapter. Notifications go into
 * the inbox like the bytes of a serial port.
 */
@SuppressLint("MissingPermission")
private class BleLink : Link() {
  @Volatile var gatt: BluetoothGatt? = null
  @Volatile var mtu = 23
  @Volatile private var gone = false
  @Volatile private var ready = false
  @Volatile private var servicesOk = false
  @Volatile private var writeOk = false
  @Volatile private var notifyChar: BluetoothGattCharacteristic? = null
  private var writeChar: BluetoothGattCharacteristic? = null
  private var writeType = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
  private val connected = CountDownLatch(1)
  private val discovered = CountDownLatch(1)
  // One GATT operation at a time: Android refuses a second one while the
  // first is still running, so each waits for its callback.
  @Volatile private var pending: CountDownLatch? = null
  private val writeLock = Object()

  val callback = object : BluetoothGattCallback() {
    override fun onConnectionStateChange(g: BluetoothGatt, status: Int, newState: Int) {
      if (newState == BluetoothProfile.STATE_CONNECTED && status == BluetoothGatt.GATT_SUCCESS) {
        connected.countDown()
      } else if (newState == BluetoothProfile.STATE_DISCONNECTED || status != BluetoothGatt.GATT_SUCCESS) {
        gone = true
        inbox.close()
        connected.countDown()
        discovered.countDown()
        pending?.countDown()
      }
    }

    override fun onServicesDiscovered(g: BluetoothGatt, status: Int) {
      servicesOk = status == BluetoothGatt.GATT_SUCCESS
      discovered.countDown()
    }

    override fun onMtuChanged(g: BluetoothGatt, value: Int, status: Int) {
      if (status == BluetoothGatt.GATT_SUCCESS) mtu = value
      pending?.countDown()
    }

    override fun onDescriptorWrite(g: BluetoothGatt, d: BluetoothGattDescriptor, status: Int) {
      pending?.countDown()
    }

    override fun onCharacteristicWrite(g: BluetoothGatt, c: BluetoothGattCharacteristic, status: Int) {
      writeOk = status == BluetoothGatt.GATT_SUCCESS
      pending?.countDown()
    }

    // Android 13 and newer hand the value over here and do not call the
    // older variant below.
    override fun onCharacteristicChanged(g: BluetoothGatt, c: BluetoothGattCharacteristic, value: ByteArray) {
      received(c, value)
    }

    @Deprecated("Android 12 and older")
    @Suppress("DEPRECATION")
    override fun onCharacteristicChanged(g: BluetoothGatt, c: BluetoothGattCharacteristic) {
      received(c, c.value ?: return)
    }
  }

  private fun received(c: BluetoothGattCharacteristic, value: ByteArray) {
    val wanted = notifyChar ?: return
    if (c.uuid == wanted.uuid && c.instanceId == wanted.instanceId) inbox.put(value, value.size)
  }

  /** Starts one GATT operation and waits for its callback. */
  private fun await(timeoutMs: Long, start: () -> Boolean): Boolean {
    val latch = CountDownLatch(1)
    pending = latch
    if (!start()) return false
    return latch.await(timeoutMs, TimeUnit.MILLISECONDS) && !gone
  }

  // The newer form of connectGatt does not exist on the oldest Android
  // versions the app runs on.
  @Suppress("DEPRECATION")
  fun connect(context: Context, device: BluetoothDevice, timeoutMs: Long) {
    gatt = device.connectGatt(context, false, callback, BluetoothDevice.TRANSPORT_LE)
      ?: throw IllegalStateException("connect")
    if (!connected.await(timeoutMs, TimeUnit.MILLISECONDS) || gone) throw IllegalStateException("connect")
    if (gatt?.discoverServices() != true) throw IllegalStateException("services")
    if (!discovered.await(10, TimeUnit.SECONDS) || gone || !servicesOk) throw IllegalStateException("services")
  }

  /** The services with their characteristics, in the order the indices of setup() count. */
  fun services(): JSArray {
    val list = JSArray()
    for (service in gatt?.services.orEmpty()) {
      val chars = JSArray()
      for (c in service.characteristics) {
        val item = JSObject()
        item.put("uuid", c.uuid.toString())
        item.put("properties", c.properties)
        chars.put(item)
      }
      val item = JSObject()
      item.put("uuid", service.uuid.toString())
      item.put("characteristics", chars)
      list.put(item)
    }
    return list
  }

  @Suppress("DEPRECATION")
  fun setup(args: BleSetupArgs) {
    val g = gatt ?: throw IllegalStateException("connect")
    val service = g.services.getOrNull(args.service) ?: throw IllegalStateException("services")
    val answers = service.characteristics.getOrNull(args.notify) ?: throw IllegalStateException("services")
    val commands = service.characteristics.getOrNull(args.write) ?: throw IllegalStateException("services")
    // A larger MTU first: answers of 20 bytes per notification are slow.
    // Without it the 23 bytes every device knows stay.
    await(3000) { g.requestMtu(args.mtu) }
    notifyChar = answers
    g.setCharacteristicNotification(answers, true)
    val cccd = answers.getDescriptor(CCCD)
    if (cccd != null) {
      val value = if (args.indicate) {
        BluetoothGattDescriptor.ENABLE_INDICATION_VALUE
      } else {
        BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
      }
      // Like on the desktop: the adapter is usable even if the
      // confirmation does not come in time.
      await(3000) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
          g.writeDescriptor(cccd, value) == BluetoothStatusCodes.SUCCESS
        } else {
          cccd.value = value
          g.writeDescriptor(cccd)
        }
      }
    }
    if (gone) throw IllegalStateException("connect")
    writeChar = commands
    writeType = if (args.noResponse) {
      BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
    } else {
      BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
    }
    ready = true
  }

  /** One value, at most the MTU long (the Rust side cuts the pieces). */
  @Suppress("DEPRECATION")
  override fun write(data: ByteArray): Boolean {
    synchronized(writeLock) {
      val g = gatt ?: return false
      val c = writeChar ?: return false
      if (!ready || gone) return false
      // The stack may still be busy with the previous value for a moment.
      repeat(3) {
        writeOk = false
        val done = await(2000) {
          if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            g.writeCharacteristic(c, data, writeType) == BluetoothStatusCodes.SUCCESS
          } else {
            c.writeType = writeType
            c.value = data
            g.writeCharacteristic(c)
          }
        }
        if (done) return writeOk
        if (gone) return false
        Thread.sleep(20)
      }
      return false
    }
  }

  override fun close() {
    ready = false
    try { gatt?.disconnect() } catch (_: Exception) {}
    try { gatt?.close() } catch (_: Exception) {}
    gatt = null
    inbox.close()
  }

  companion object {
    val CCCD: UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")
  }
}

/** A USB serial adapter through usb-serial-for-android. */
private class UsbLink(private val port: UsbSerialPort) : Link(), SerialInputOutputManager.Listener {
  private val io = SerialInputOutputManager(port, this)

  init {
    io.start()
  }

  override fun onNewData(data: ByteArray) {
    inbox.put(data, data.size)
  }

  override fun onRunError(e: Exception) {
    inbox.close()
  }

  override fun write(data: ByteArray): Boolean =
    try {
      port.write(data, 500)
      true
    } catch (_: Exception) {
      false
    }

  override fun close() {
    try { io.stop() } catch (_: Exception) {}
    try { port.close() } catch (_: Exception) {}
    inbox.close()
  }
}

@TauriPlugin(
  permissions = [
    Permission(
      strings = [Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT],
      alias = "bluetooth"
    ),
    // Up to Android 11 a Bluetooth LE scan finds nothing without it.
    Permission(
      strings = [Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION],
      alias = "location"
    )
  ]
)
class AdapterPlugin(private val activity: Activity) : Plugin(activity) {
  private val links = ConcurrentHashMap<Int, Link>()
  private val nextId = AtomicInteger(1)
  // Opening and reading block; the commands arrive on the main thread.
  private val pool = Executors.newCachedThreadPool()

  private fun base64(bytes: ByteArray): String = Base64.encodeToString(bytes, Base64.NO_WRAP)

  private fun background(invoke: Invoke, work: () -> Unit) {
    pool.execute {
      try {
        work()
      } catch (e: Exception) {
        invoke.reject(e.message ?: e.toString())
      }
    }
  }

  private fun remember(link: Link): Int {
    val id = nextId.getAndIncrement()
    links[id] = link
    return id
  }

  // --- Bluetooth ----------------------------------------------------------------------------

  /** Android 12 and newer ask at run time; older versions grant Bluetooth with the app. */
  private fun bluetoothGranted(): Boolean =
    Build.VERSION.SDK_INT < Build.VERSION_CODES.S ||
      getPermissionState("bluetooth") == PermissionState.GRANTED

  /** Android 12 and newer scan for Bluetooth LE with "nearby devices"; older ones need the location. */
  private fun locationNeeded(): Boolean =
    Build.VERSION.SDK_INT < Build.VERSION_CODES.S &&
      getPermissionState("location") != PermissionState.GRANTED

  private fun locationOn(): Boolean {
    val manager = activity.getSystemService(Context.LOCATION_SERVICE) as? LocationManager ?: return false
    return LocationManagerCompat.isLocationEnabled(manager)
  }

  private fun bluetoothState(): JSObject {
    val adapter = (activity.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter
    val result = JSObject()
    result.put("available", adapter != null)
    result.put("needsPermission", Build.VERSION.SDK_INT >= Build.VERSION_CODES.S)
    result.put("granted", bluetoothGranted())
    val enabled = try { adapter?.isEnabled == true } catch (_: SecurityException) { false }
    result.put("enabled", enabled)
    result.put("locationNeeded", locationNeeded())
    return result
  }

  @Command
  fun bluetoothStatus(invoke: Invoke) {
    invoke.resolve(bluetoothState())
  }

  @Command
  fun requestBluetooth(invoke: Invoke) {
    if (!bluetoothGranted()) {
      requestPermissionForAlias("bluetooth", invoke, "bluetoothPermissionDone")
    } else if (locationNeeded()) {
      requestPermissionForAlias("location", invoke, "bluetoothPermissionDone")
    } else {
      invoke.resolve(bluetoothState())
    }
  }

  @PermissionCallback
  fun bluetoothPermissionDone(invoke: Invoke) {
    invoke.resolve(bluetoothState())
  }

  /** The paired devices; adapters are paired once in the system settings (PIN 1234 or 0000). */
  @SuppressLint("MissingPermission")
  @Command
  fun pairedDevices(invoke: Invoke) {
    if (!bluetoothGranted()) {
      invoke.reject("permission")
      return
    }
    val adapter = (activity.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter
    val list = JSArray()
    try {
      for (device in adapter?.bondedDevices.orEmpty()) {
        // Classic or dual mode only: a pure LE device has no serial port.
        if (device.type == BluetoothDevice.DEVICE_TYPE_LE) continue
        val item = JSObject()
        item.put("address", device.address)
        item.put("name", device.name ?: "")
        list.put(item)
      }
    } catch (e: SecurityException) {
      invoke.reject("permission")
      return
    }
    val result = JSObject()
    result.put("devices", list)
    invoke.resolve(result)
  }

  @SuppressLint("MissingPermission")
  @Command
  fun openBluetooth(invoke: Invoke) {
    val args = invoke.parseArgs(BluetoothArgs::class.java)
    background(invoke) {
      if (!bluetoothGranted()) throw IllegalStateException("permission")
      val adapter = (activity.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager).adapter
        ?: throw IllegalStateException("no Bluetooth")
      val device = adapter.getRemoteDevice(args.address)
      try { adapter.cancelDiscovery() } catch (_: SecurityException) {}
      val spp = UUID.fromString("00001101-0000-1000-8000-00805F9B34FB")
      // Cheap clones often only answer on one of these three ways.
      val attempts: List<() -> BluetoothSocket> = listOf(
        { device.createRfcommSocketToServiceRecord(spp) },
        { device.createInsecureRfcommSocketToServiceRecord(spp) },
        {
          device.javaClass.getMethod("createRfcommSocket", Int::class.javaPrimitiveType)
            .invoke(device, 1) as BluetoothSocket
        },
      )
      var last: Exception? = null
      for (attempt in attempts) {
        var socket: BluetoothSocket? = null
        try {
          socket = attempt()
          socket.connect()
          val result = JSObject()
          result.put("id", remember(BluetoothLink(socket)))
          invoke.resolve(result)
          return@background
        } catch (e: Exception) {
          last = e
          try { socket?.close() } catch (_: Exception) {}
        }
      }
      throw last ?: IllegalStateException("connect")
    }
  }

  // --- Bluetooth LE ---------------------------------------------------------------------------

  /**
   * Bluetooth LE devices in range, for the given time: address, name,
   * signal strength and the services they advertise. Only listens; nothing
   * is sent to any device.
   */
  @SuppressLint("MissingPermission")
  @Command
  fun bleScan(invoke: Invoke) {
    val args = invoke.parseArgs(BleScanArgs::class.java)
    background(invoke) {
      if (!bluetoothGranted()) throw IllegalStateException("permission")
      if (locationNeeded()) throw IllegalStateException("location-permission")
      if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S && !locationOn()) {
        throw IllegalStateException("location-off")
      }
      val adapter = (activity.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter
        ?: throw IllegalStateException("no-bluetooth")
      if (!adapter.isEnabled) throw IllegalStateException("off")
      val scanner = adapter.bluetoothLeScanner ?: throw IllegalStateException("off")
      val seen = ConcurrentHashMap<String, ScanResult>()
      val failed = AtomicInteger(0)
      val stopped = CountDownLatch(1)
      val callback = object : ScanCallback() {
        override fun onScanResult(callbackType: Int, result: ScanResult) {
          seen[result.device.address] = result
        }

        override fun onBatchScanResults(results: MutableList<ScanResult>) {
          for (r in results) seen[r.device.address] = r
        }

        override fun onScanFailed(errorCode: Int) {
          failed.set(errorCode)
          stopped.countDown()
        }
      }
      val settings = ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()
      scanner.startScan(null, settings, callback)
      stopped.await(args.timeout.coerceIn(1000, 30000), TimeUnit.MILLISECONDS)
      try { scanner.stopScan(callback) } catch (_: Exception) {}
      if (failed.get() != 0 && seen.isEmpty()) throw IllegalStateException("scan-failed ${failed.get()}")
      val list = JSArray()
      for (r in seen.values) {
        val name = r.scanRecord?.deviceName
          ?: try { r.device.name } catch (_: SecurityException) { null }
          ?: ""
        val uuids = JSArray()
        for (u in r.scanRecord?.serviceUuids.orEmpty()) uuids.put(u.uuid.toString())
        val item = JSObject()
        item.put("address", r.device.address)
        item.put("name", name.trim())
        item.put("rssi", r.rssi)
        item.put("services", uuids)
        list.put(item)
      }
      val result = JSObject()
      result.put("devices", list)
      invoke.resolve(result)
    }
  }

  /**
   * Connects and reads the services, without writing anything. The link is
   * kept under its number; the Rust side then either sets it up or closes it.
   */
  @SuppressLint("MissingPermission")
  @Command
  fun bleConnect(invoke: Invoke) {
    val args = invoke.parseArgs(BleConnectArgs::class.java)
    background(invoke) {
      if (!bluetoothGranted()) throw IllegalStateException("permission")
      val adapter = (activity.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter
        ?: throw IllegalStateException("no-bluetooth")
      if (!adapter.isEnabled) throw IllegalStateException("off")
      val device = adapter.getRemoteDevice(args.address)
      // The first attempt often ends at once in the stack's error 133; a
      // second one after a short pause usually works. A device that is
      // simply not there gets no second wait.
      var last: Exception? = null
      repeat(2) {
        val link = BleLink()
        val started = System.currentTimeMillis()
        try {
          link.connect(activity, device, args.timeout)
          val name = try { device.name ?: "" } catch (_: SecurityException) { "" }
          val result = JSObject()
          result.put("id", remember(link))
          result.put("name", name)
          result.put("services", link.services())
          invoke.resolve(result)
          return@background
        } catch (e: Exception) {
          last = e
          link.close()
          if (System.currentTimeMillis() - started >= args.timeout) throw e
          Thread.sleep(500)
        }
      }
      throw last ?: IllegalStateException("connect")
    }
  }

  /** Notifications on, a larger MTU, and from then on writes are allowed. */
  @Command
  fun bleSetup(invoke: Invoke) {
    val args = invoke.parseArgs(BleSetupArgs::class.java)
    val link = links[args.id] as? BleLink
    background(invoke) {
      if (link == null) throw IllegalStateException("connect")
      link.setup(args)
      val result = JSObject()
      result.put("mtu", link.mtu)
      invoke.resolve(result)
    }
  }

  // --- USB ----------------------------------------------------------------------------------

  private fun usbManager() = activity.getSystemService(Context.USB_SERVICE) as UsbManager

  @Command
  fun usbDevices(invoke: Invoke) {
    val list = JSArray()
    for (driver in UsbSerialProber.getDefaultProber().findAllDrivers(usbManager())) {
      val device = driver.device
      val item = JSObject()
      item.put("device", device.deviceName)
      item.put("vid", device.vendorId)
      item.put("pid", device.productId)
      item.put("product", device.productName ?: "")
      item.put("driver", driver.javaClass.simpleName.removeSuffix("SerialDriver"))
      list.put(item)
    }
    val result = JSObject()
    result.put("devices", list)
    invoke.resolve(result)
  }

  /** Asks the user for access to one USB device and waits for the answer. */
  private fun usbPermission(manager: UsbManager, device: UsbDevice): Boolean {
    if (manager.hasPermission(device)) return true
    val action = activity.packageName + ".USB_PERMISSION"
    val answered = CountDownLatch(1)
    val receiver = object : BroadcastReceiver() {
      override fun onReceive(context: Context, intent: Intent) {
        answered.countDown()
      }
    }
    ContextCompat.registerReceiver(
      activity, receiver, IntentFilter(action), ContextCompat.RECEIVER_NOT_EXPORTED
    )
    try {
      val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0
      val intent = PendingIntent.getBroadcast(
        activity, 0, Intent(action).setPackage(activity.packageName), flags
      )
      manager.requestPermission(device, intent)
      answered.await(60, TimeUnit.SECONDS)
    } finally {
      try { activity.unregisterReceiver(receiver) } catch (_: Exception) {}
    }
    return manager.hasPermission(device)
  }

  @Command
  fun openUsb(invoke: Invoke) {
    val args = invoke.parseArgs(UsbArgs::class.java)
    background(invoke) {
      val manager = usbManager()
      val driver = UsbSerialProber.getDefaultProber().findAllDrivers(manager)
        .firstOrNull { it.device.deviceName == args.device }
        ?: throw IllegalStateException("not found")
      if (!usbPermission(manager, driver.device)) throw IllegalStateException("permission")
      val connection = manager.openDevice(driver.device) ?: throw IllegalStateException("open")
      val port = driver.ports.first()
      port.open(connection)
      port.setParameters(args.baud, 8, UsbSerialPort.STOPBITS_1, UsbSerialPort.PARITY_NONE)
      try {
        port.dtr = true
        port.rts = true
      } catch (_: Exception) {
        // Not every chip has the lines.
      }
      val result = JSObject()
      result.put("id", remember(UsbLink(port)))
      invoke.resolve(result)
    }
  }

  // --- An open link -------------------------------------------------------------------------

  @Command
  fun write(invoke: Invoke) {
    val args = invoke.parseArgs(WriteArgs::class.java)
    val link = links[args.id]
    background(invoke) {
      val ok = link?.write(Base64.decode(args.data, Base64.DEFAULT)) ?: false
      val result = JSObject()
      result.put("ok", ok)
      invoke.resolve(result)
    }
  }

  @Command
  fun read(invoke: Invoke) {
    val args = invoke.parseArgs(ReadArgs::class.java)
    val link = links[args.id]
    background(invoke) {
      val bytes = link?.inbox?.take(args.timeout) ?: ByteArray(0)
      val result = JSObject()
      result.put("data", base64(bytes))
      result.put("closed", link == null || link.inbox.closed)
      invoke.resolve(result)
    }
  }

  @Command
  fun flush(invoke: Invoke) {
    val args = invoke.parseArgs(LinkArgs::class.java)
    links[args.id]?.inbox?.clear()
    invoke.resolve()
  }

  @Command
  fun close(invoke: Invoke) {
    val args = invoke.parseArgs(LinkArgs::class.java)
    val link = links.remove(args.id)
    background(invoke) {
      link?.close()
      invoke.resolve()
    }
  }

  // --- WLAN adapters ------------------------------------------------------------------------

  /**
   * A WLAN adapter has no internet, so Android keeps the mobile data as
   * the default way out and a socket to 192.168.0.10 would go there. While
   * connected, the app's sockets use the WLAN.
   */
  @Command
  fun bindWifi(invoke: Invoke) {
    val args = invoke.parseArgs(WifiArgs::class.java)
    val manager = activity.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
    var bound = false
    try {
      if (args.on) {
        @Suppress("DEPRECATION")
        val wifi = manager.allNetworks.firstOrNull {
          manager.getNetworkCapabilities(it)?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
        }
        if (wifi != null) bound = manager.bindProcessToNetwork(wifi)
      } else {
        manager.bindProcessToNetwork(null)
      }
    } catch (_: Exception) {
      bound = false
    }
    val result = JSObject()
    result.put("bound", bound)
    invoke.resolve(result)
  }

  // --- Files of the system dialog ---------------------------------------------------------

  @Command
  fun writeUri(invoke: Invoke) {
    val args = invoke.parseArgs(UriArgs::class.java)
    background(invoke) {
      activity.contentResolver.openOutputStream(Uri.parse(args.uri), "wt").use { out ->
        out ?: throw IllegalStateException("open")
        out.write(Base64.decode(args.data, Base64.DEFAULT))
      }
      invoke.resolve()
    }
  }

  @Command
  fun readUri(invoke: Invoke) {
    val args = invoke.parseArgs(UriArgs::class.java)
    background(invoke) {
      val uri = Uri.parse(args.uri)
      val bytes = activity.contentResolver.openInputStream(uri).use { input ->
        input?.readBytes() ?: throw IllegalStateException("open")
      }
      var name = ""
      activity.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
        if (it.moveToFirst()) name = it.getString(0) ?: ""
      }
      val result = JSObject()
      result.put("name", name)
      result.put("data", base64(bytes))
      invoke.resolve(result)
    }
  }

  override fun onDestroy(activity: AppCompatActivity) {
    for (link in links.values) link.close()
    links.clear()
    super.onDestroy(activity)
  }
}
