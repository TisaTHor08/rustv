//! Service Bluetooth RFCOMM (secours quand le Wi-Fi bloque : isolation des clients, etc.).
//! Le téléphone doit avoir été appairé une fois au PC dans les réglages Bluetooth de Windows.
use crate::core::Core;
use crate::server::serve_session;
use rustv_protocol::session::Session;
use rustv_protocol::RFCOMM_UUID;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use windows::core::{Result as WinResult, GUID};
use windows::Devices::Bluetooth::BluetoothAdapter;
use windows::Devices::Bluetooth::Rfcomm::{RfcommServiceId, RfcommServiceProvider};
use windows::Foundation::TypedEventHandler;
use windows::Networking::Sockets::{
    SocketProtectionLevel, StreamSocket, StreamSocketListener, StreamSocketListenerConnectionReceivedEventArgs,
};
use windows::Storage::Streams::{DataReader, DataWriter, InputStreamOptions};

fn parse_guid(s: &str) -> GUID {
    let hex: String = s.chars().filter(|c| *c != '-').collect();
    GUID::from_u128(u128::from_str_radix(&hex, 16).expect("UUID valide"))
}

/// Adresse Bluetooth du PC « AA:BB:CC:DD:EE:FF », ou None s'il n'y a pas de Bluetooth.
pub fn adapter_address() -> Option<String> {
    let a = BluetoothAdapter::GetDefaultAsync().ok()?.join().ok()?;
    let n = a.BluetoothAddress().ok()?;
    Some((0..6).rev().map(|i| format!("{:02X}", (n >> (i * 8)) & 0xFF)).collect::<Vec<_>>().join(":"))
}

struct Inner {
    socket: StreamSocket,
    reader: Mutex<DataReader>,
    writer: Mutex<DataWriter>,
}
// Les objets WinRT utilisés ici sont agiles (utilisables depuis n'importe quel thread).
unsafe impl Send for Inner {}
unsafe impl Sync for Inner {}

/// Flux partageable : une moitié lit, l'autre écrit, chacune derrière son propre verrou.
#[derive(Clone)]
struct BtStream(Arc<Inner>);

fn io_err(e: windows::core::Error) -> io::Error {
    io::Error::new(io::ErrorKind::ConnectionAborted, e.message())
}

impl BtStream {
    fn new(socket: StreamSocket) -> WinResult<Self> {
        let reader = DataReader::CreateDataReader(&socket.InputStream()?)?;
        reader.SetInputStreamOptions(InputStreamOptions::Partial)?; // rend la main dès que des octets arrivent
        let writer = DataWriter::CreateDataWriter(&socket.OutputStream()?)?;
        Ok(Self(Arc::new(Inner { socket, reader: Mutex::new(reader), writer: Mutex::new(writer) })))
    }
    fn close(&self) {
        let _ = self.0.socket.Close();
    }
}

impl Read for BtStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let r = self.0.reader.lock().unwrap();
        let n = r.LoadAsync(buf.len() as u32).map_err(io_err)?.join().map_err(io_err)? as usize;
        if n == 0 {
            return Ok(0);
        }
        r.ReadBytes(&mut buf[..n]).map_err(io_err)?;
        Ok(n)
    }
}

impl Write for BtStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let w = self.0.writer.lock().unwrap();
        w.WriteBytes(buf).map_err(io_err)?;
        w.StoreAsync().map_err(io_err)?.join().map_err(io_err)?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn run(core: Arc<Core>) -> WinResult<()> {
    let id = RfcommServiceId::FromUuid(parse_guid(RFCOMM_UUID))?;
    let provider = RfcommServiceProvider::CreateAsync(&id)?.join()?;
    let listener = StreamSocketListener::new()?;

    let c = core.clone();
    listener.ConnectionReceived(&TypedEventHandler::new(move |_: windows::core::Ref<StreamSocketListener>, args: windows::core::Ref<StreamSocketListenerConnectionReceivedEventArgs>| {
        let socket = args.ok()?.Socket()?;
        let core = c.clone();
        std::thread::spawn(move || {
            let Ok(stream) = BtStream::new(socket) else { return };
            let lookup = {
                let core = core.clone();
                move |id: &str| core.cfg.lock().unwrap().key_of(id)
            };
            let Ok(sess) = Session::accept(stream.clone(), lookup) else {
                stream.close();
                return;
            };
            let closer = stream.clone();
            serve_session(core, sess, stream, move || closer.close());
        });
        Ok(())
    }))?;

    listener
        .BindServiceNameWithProtectionLevelAsync(&provider.ServiceId()?.AsString()?, SocketProtectionLevel::BluetoothEncryptionAllowNullAuthentication)?
        .join()?;
    provider.StartAdvertising(&listener)?;

    // Garde le fournisseur et l'écouteur en vie.
    loop {
        std::thread::park();
        let _ = (&provider, &listener);
    }
}
