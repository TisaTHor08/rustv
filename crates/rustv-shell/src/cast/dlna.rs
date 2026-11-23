//! DLNA / UPnP MediaRenderer : description, SCPD, SOAP (AVTransport, RenderingControl, ConnectionManager),
//! DIDL-Lite, REL_TIME et événements GENA. Tout le texte (XML, en-têtes) est produit par des fonctions pures.
use super::http::{Request, Response};
use super::{Host, PlayerSnap, Shared};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::net::IpAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const MAX_URL: usize = 2048;
const MAX_ARG: usize = 64 * 1024;
const MAX_SUBS: usize = 16;
const MAX_NEW_SUBS_PER_MIN: usize = 20;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Service {
    Avt,
    Rc,
    Cm,
}

impl Service {
    pub fn urn(self) -> &'static str {
        match self {
            Service::Avt => "urn:schemas-upnp-org:service:AVTransport:1",
            Service::Rc => "urn:schemas-upnp-org:service:RenderingControl:1",
            Service::Cm => "urn:schemas-upnp-org:service:ConnectionManager:1",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Service::Avt => "urn:upnp-org:serviceId:AVTransport",
            Service::Rc => "urn:upnp-org:serviceId:RenderingControl",
            Service::Cm => "urn:upnp-org:serviceId:ConnectionManager",
        }
    }
    pub fn path(self) -> &'static str {
        match self {
            Service::Avt => "avt",
            Service::Rc => "rc",
            Service::Cm => "cm",
        }
    }
    pub fn from_path(p: &str) -> Option<Self> {
        [Service::Avt, Service::Rc, Service::Cm].into_iter().find(|s| s.path() == p)
    }
}

// ---------- utilitaires purs ----------

pub fn xml_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            c if c.is_control() && c != '\n' && c != '\t' => {}
            c => o.push(c),
        }
    }
    o
}

/// Secondes → `H:MM:SS` (format REL_TIME d'UPnP).
pub fn fmt_reltime(secs: f64) -> String {
    let s = if secs.is_finite() && secs > 0.0 { secs as u64 } else { 0 };
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// `H+:MM:SS[.F+]` (signe facultatif) → secondes. `None` si invalide ou « NOT_IMPLEMENTED ».
pub fn parse_reltime(s: &str) -> Option<f64> {
    let s = s.trim();
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let mut it = s.split(':');
    let (h, m, sec) = (it.next()?, it.next()?, it.next()?);
    if it.next().is_some() || h.is_empty() || h.len() > 6 || !h.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let m = (m.len() == 2).then(|| m.parse::<u32>().ok()).flatten().filter(|m| *m < 60)?;
    let (si, sf) = sec.split_once('.').unwrap_or((sec, ""));
    let si = (si.len() == 2).then(|| si.parse::<u32>().ok()).flatten().filter(|x| *x < 60)?;
    if sf.len() > 6 || !sf.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let frac = if sf.is_empty() { 0.0 } else { format!("0.{sf}").parse::<f64>().ok()? };
    let v = h.parse::<f64>().ok()? * 3600.0 + f64::from(m) * 60.0 + f64::from(si) + frac;
    Some(if neg { -v } else { v })
}

/// Valide l'URL d'un média : http(s) uniquement, jamais file:// ni chemin local ni identifiants intégrés.
pub fn validate_media_url(u: &str) -> Result<String, &'static str> {
    let u = u.trim();
    if u.is_empty() || u.len() > MAX_URL || u.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("invalid url");
    }
    let p: tauri::Url = u.parse().map_err(|_| "invalid url")?;
    if !matches!(p.scheme(), "http" | "https") || p.host_str().is_none() || !p.username().is_empty() || p.password().is_some() {
        return Err("invalid url");
    }
    Ok(p.to_string())
}

#[derive(Default, Debug, PartialEq)]
pub struct Didl {
    pub title: String,
    pub class: String,
    pub album_art: String,
    pub mime: String,
}

fn clean(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_string()
}

/// Lit un DIDL-Lite (titre, classe UPnP, jaquette, type MIME du premier `res`). Un DOCTYPE fait tout ignorer.
pub fn parse_didl(xml: &str) -> Didl {
    let mut d = Didl::default();
    let mut rd = Reader::from_str(xml);
    rd.config_mut().trim_text(true);
    let mut cur = "";
    for _ in 0..20_000 {
        match rd.read_event() {
            Ok(Event::DocType(_)) => return Didl::default(),
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                cur = match name.as_str() {
                    "title" if d.title.is_empty() => "title",
                    "class" if d.class.is_empty() => "class",
                    "albumArtURI" if d.album_art.is_empty() => "art",
                    _ => "",
                };
                if name == "res" && d.mime.is_empty() {
                    for a in e.attributes().flatten() {
                        if a.key.local_name().as_ref() == b"protocolInfo" {
                            if let Ok(v) = a.unescape_value() {
                                // protocole:réseau:type-mime:infos
                                d.mime = v.split(':').nth(2).map(|m| clean(m, 80).to_ascii_lowercase()).unwrap_or_default();
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if let Ok(v) = t.unescape() {
                    match cur {
                        "title" => d.title = clean(&v, 200),
                        "class" => d.class = clean(&v, 100),
                        "art" => d.album_art = validate_media_url(&v).unwrap_or_default(),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(_)) => cur = "",
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    d
}

/// Nature du média : `video`, `audio` ou `image` (non pris en charge), d'après la classe, le MIME puis l'extension.
pub fn classify(d: &Didl, url: &str) -> &'static str {
    let c = d.class.to_ascii_lowercase();
    if c.contains("imageitem") {
        return "image";
    }
    if c.contains("audioitem") {
        return "audio";
    }
    if c.contains("videoitem") {
        return "video";
    }
    if let Some((major, _)) = d.mime.split_once('/') {
        match major {
            "image" => return "image",
            "audio" => return "audio",
            "video" => return "video",
            _ => {}
        }
    }
    let path = url.split(['?', '#']).next().unwrap_or("");
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" => "image",
        "mp3" | "flac" | "wav" | "aac" | "m4a" | "ogg" | "oga" | "opus" | "wma" => "audio",
        _ => "video",
    }
}

fn title_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or("");
    let last = path.rsplit('/').next().unwrap_or("");
    let t = clean(last, 120);
    if t.is_empty() {
        "RusTV Cast".into()
    } else {
        t
    }
}

// ---------- SOAP ----------

#[derive(Debug, PartialEq)]
pub struct Soap {
    pub action: String,
    pub args: Vec<(String, String)>,
}

/// Analyse une enveloppe SOAP UPnP. Refuse DOCTYPE (donc toute définition d'entité), borne les arguments.
pub fn parse_soap(body: &str) -> Result<Soap, &'static str> {
    let mut rd = Reader::from_str(body);
    rd.config_mut().trim_text(true);
    let (mut depth, mut action, mut args) = (0u32, String::new(), Vec::<(String, String)>::new());
    let mut cur: Option<(String, String)> = None;
    for _ in 0..5_000 {
        match rd.read_event() {
            Ok(Event::DocType(_)) => return Err("doctype"),
            Ok(Event::Start(e)) => {
                depth += 1;
                let n = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                match depth {
                    1 if n != "Envelope" => return Err("not soap"),
                    2 if n != "Body" => return Err("not soap"),
                    3 => action = n,
                    4 => cur = Some((n, String::new())),
                    d if d > 4 => return Err("too deep"),
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let n = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                match depth {
                    2 => action = n,
                    3 if args.len() < 32 => args.push((n, String::new())),
                    3 => return Err("too many args"),
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                if let Some((_, v)) = cur.as_mut() {
                    v.push_str(&t.unescape().map_err(|_| "bad xml")?);
                    if v.len() > MAX_ARG {
                        return Err("arg too large");
                    }
                }
            }
            Ok(Event::CData(t)) => {
                if let Some((_, v)) = cur.as_mut() {
                    v.push_str(&String::from_utf8_lossy(&t.into_inner()));
                    if v.len() > MAX_ARG {
                        return Err("arg too large");
                    }
                }
            }
            Ok(Event::End(_)) => {
                if depth == 4 {
                    if let Some(a) = cur.take() {
                        if args.len() >= 32 {
                            return Err("too many args");
                        }
                        args.push(a);
                    }
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Err(_) => return Err("bad xml"),
            _ => {}
        }
    }
    if action.is_empty() || action.len() > 64 || !action.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("no action");
    }
    Ok(Soap { action, args })
}

pub fn soap_response(svc: Service, action: &str, out: &[(String, String)]) -> String {
    let mut s = format!(
        "<?xml version=\"1.0\"?>\r\n<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{action}Response xmlns:u=\"{}\">",
        svc.urn()
    );
    for (k, v) in out {
        s.push_str(&format!("<{k}>{}</{k}>", xml_escape(v)));
    }
    s.push_str(&format!("</u:{action}Response></s:Body></s:Envelope>"));
    s
}

pub fn soap_fault(code: u16, desc: &str) -> String {
    format!(
        "<?xml version=\"1.0\"?>\r\n<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><s:Fault><faultcode>s:Client</faultcode><faultstring>UPnPError</faultstring><detail><UPnPError xmlns=\"urn:schemas-upnp-org:control-1-0\"><errorCode>{code}</errorCode><errorDescription>{}</errorDescription></UPnPError></detail></s:Fault></s:Body></s:Envelope>",
        xml_escape(desc)
    )
}

// ---------- description de l'appareil et SCPD ----------

pub fn device_xml(name: &str, uuid: &str) -> String {
    let mut s = format!(
        "<?xml version=\"1.0\"?>\r\n<root xmlns=\"urn:schemas-upnp-org:device-1-0\" xmlns:dlna=\"urn:schemas-dlna-org:device-1-0\"><specVersion><major>1</major><minor>0</minor></specVersion><device><dlna:X_DLNADOC>DMR-1.50</dlna:X_DLNADOC><deviceType>urn:schemas-upnp-org:device:MediaRenderer:1</deviceType><friendlyName>{}</friendlyName><manufacturer>RusTV</manufacturer><modelDescription>RusTV TV mode</modelDescription><modelName>RusTV</modelName><modelNumber>{}</modelNumber><UDN>uuid:{uuid}</UDN><serviceList>",
        xml_escape(name),
        env!("CARGO_PKG_VERSION")
    );
    for sv in [Service::Avt, Service::Rc, Service::Cm] {
        let p = sv.path();
        s.push_str(&format!(
            "<service><serviceType>{}</serviceType><serviceId>{}</serviceId><SCPDURL>/dlna/{p}/scpd.xml</SCPDURL><controlURL>/dlna/{p}/control</controlURL><eventSubURL>/dlna/{p}/event</eventSubURL></service>",
            sv.urn(),
            sv.id()
        ));
    }
    s.push_str("</serviceList></device></root>");
    s
}

type Arg = (&'static str, &'static str, &'static str);
type Act = (&'static str, &'static [Arg]);
/// nom, type, évènementielle, valeurs permises
type Var = (&'static str, &'static str, bool, &'static [&'static str]);

const IID: Arg = ("InstanceID", "in", "A_ARG_TYPE_InstanceID");
const AVT_ACTIONS: &[Act] = &[
    ("SetAVTransportURI", &[IID, ("CurrentURI", "in", "AVTransportURI"), ("CurrentURIMetaData", "in", "AVTransportURIMetaData")]),
    ("Play", &[IID, ("Speed", "in", "TransportPlaySpeed")]),
    ("Pause", &[IID]),
    ("Stop", &[IID]),
    ("Seek", &[IID, ("Unit", "in", "A_ARG_TYPE_SeekMode"), ("Target", "in", "A_ARG_TYPE_SeekTarget")]),
    ("Next", &[IID]),
    ("Previous", &[IID]),
    ("GetTransportInfo", &[IID, ("CurrentTransportState", "out", "TransportState"), ("CurrentTransportStatus", "out", "TransportStatus"), ("CurrentSpeed", "out", "TransportPlaySpeed")]),
    (
        "GetPositionInfo",
        &[
            IID,
            ("Track", "out", "CurrentTrack"),
            ("TrackDuration", "out", "CurrentTrackDuration"),
            ("TrackMetaData", "out", "CurrentTrackMetaData"),
            ("TrackURI", "out", "CurrentTrackURI"),
            ("RelTime", "out", "RelativeTimePosition"),
            ("AbsTime", "out", "AbsoluteTimePosition"),
            ("RelCount", "out", "RelativeCounterPosition"),
            ("AbsCount", "out", "AbsoluteCounterPosition"),
        ],
    ),
    (
        "GetMediaInfo",
        &[
            IID,
            ("NrTracks", "out", "NumberOfTracks"),
            ("MediaDuration", "out", "CurrentMediaDuration"),
            ("CurrentURI", "out", "AVTransportURI"),
            ("CurrentURIMetaData", "out", "AVTransportURIMetaData"),
            ("NextURI", "out", "NextAVTransportURI"),
            ("NextURIMetaData", "out", "NextAVTransportURIMetaData"),
            ("PlayMedium", "out", "PlaybackStorageMedium"),
            ("RecordMedium", "out", "RecordStorageMedium"),
            ("WriteStatus", "out", "RecordMediumWriteStatus"),
        ],
    ),
    ("GetCurrentTransportActions", &[IID, ("Actions", "out", "CurrentTransportActions")]),
    ("GetDeviceCapabilities", &[IID, ("PlayMedia", "out", "PossiblePlaybackStorageMedia"), ("RecMedia", "out", "PossibleRecordStorageMedia"), ("RecQualityModes", "out", "PossibleRecordQualityModes")]),
    ("GetTransportSettings", &[IID, ("PlayMode", "out", "CurrentPlayMode"), ("RecQualityMode", "out", "CurrentRecordQualityMode")]),
];
const AVT_VARS: &[Var] = &[
    ("TransportState", "string", false, &["STOPPED", "PLAYING", "PAUSED_PLAYBACK", "TRANSITIONING", "NO_MEDIA_PRESENT"]),
    ("TransportStatus", "string", false, &["OK", "ERROR_OCCURRED"]),
    ("PlaybackStorageMedium", "string", false, &[]),
    ("RecordStorageMedium", "string", false, &[]),
    ("PossiblePlaybackStorageMedia", "string", false, &[]),
    ("PossibleRecordStorageMedia", "string", false, &[]),
    ("CurrentPlayMode", "string", false, &["NORMAL"]),
    ("TransportPlaySpeed", "string", false, &["1"]),
    ("RecordMediumWriteStatus", "string", false, &[]),
    ("CurrentRecordQualityMode", "string", false, &[]),
    ("PossibleRecordQualityModes", "string", false, &[]),
    ("NumberOfTracks", "ui4", false, &[]),
    ("CurrentTrack", "ui4", false, &[]),
    ("CurrentTrackDuration", "string", false, &[]),
    ("CurrentMediaDuration", "string", false, &[]),
    ("CurrentTrackMetaData", "string", false, &[]),
    ("CurrentTrackURI", "string", false, &[]),
    ("AVTransportURI", "string", false, &[]),
    ("AVTransportURIMetaData", "string", false, &[]),
    ("NextAVTransportURI", "string", false, &[]),
    ("NextAVTransportURIMetaData", "string", false, &[]),
    ("RelativeTimePosition", "string", false, &[]),
    ("AbsoluteTimePosition", "string", false, &[]),
    ("RelativeCounterPosition", "i4", false, &[]),
    ("AbsoluteCounterPosition", "i4", false, &[]),
    ("CurrentTransportActions", "string", false, &[]),
    ("LastChange", "string", true, &[]),
    ("A_ARG_TYPE_SeekMode", "string", false, &["REL_TIME", "ABS_TIME", "TRACK_NR"]),
    ("A_ARG_TYPE_SeekTarget", "string", false, &[]),
    ("A_ARG_TYPE_InstanceID", "ui4", false, &[]),
];
const CH: Arg = ("Channel", "in", "A_ARG_TYPE_Channel");
const RC_ACTIONS: &[Act] = &[
    ("ListPresets", &[IID, ("CurrentPresetNameList", "out", "PresetNameList")]),
    ("SelectPreset", &[IID, ("PresetName", "in", "A_ARG_TYPE_PresetName")]),
    ("GetVolume", &[IID, CH, ("CurrentVolume", "out", "Volume")]),
    ("SetVolume", &[IID, CH, ("DesiredVolume", "in", "Volume")]),
    ("GetMute", &[IID, CH, ("CurrentMute", "out", "Mute")]),
    ("SetMute", &[IID, CH, ("DesiredMute", "in", "Mute")]),
];
const RC_VARS: &[Var] = &[
    ("PresetNameList", "string", false, &[]),
    ("LastChange", "string", true, &[]),
    ("Mute", "boolean", false, &[]),
    ("Volume", "ui2", false, &[]),
    ("A_ARG_TYPE_Channel", "string", false, &["Master"]),
    ("A_ARG_TYPE_InstanceID", "ui4", false, &[]),
    ("A_ARG_TYPE_PresetName", "string", false, &["FactoryDefaults"]),
];
const CM_ACTIONS: &[Act] = &[
    ("GetProtocolInfo", &[("Source", "out", "SourceProtocolInfo"), ("Sink", "out", "SinkProtocolInfo")]),
    ("GetCurrentConnectionIDs", &[("ConnectionIDs", "out", "CurrentConnectionIDs")]),
    (
        "GetCurrentConnectionInfo",
        &[
            ("ConnectionID", "in", "A_ARG_TYPE_ConnectionID"),
            ("RcsID", "out", "A_ARG_TYPE_RcsID"),
            ("AVTransportID", "out", "A_ARG_TYPE_AVTransportID"),
            ("ProtocolInfo", "out", "A_ARG_TYPE_ProtocolInfo"),
            ("PeerConnectionManager", "out", "A_ARG_TYPE_ConnectionManager"),
            ("PeerConnectionID", "out", "A_ARG_TYPE_ConnectionID"),
            ("Direction", "out", "A_ARG_TYPE_Direction"),
            ("Status", "out", "A_ARG_TYPE_ConnectionStatus"),
        ],
    ),
];
const CM_VARS: &[Var] = &[
    ("SourceProtocolInfo", "string", true, &[]),
    ("SinkProtocolInfo", "string", true, &[]),
    ("CurrentConnectionIDs", "string", true, &[]),
    ("A_ARG_TYPE_ConnectionStatus", "string", false, &["OK", "Unknown"]),
    ("A_ARG_TYPE_ConnectionManager", "string", false, &[]),
    ("A_ARG_TYPE_Direction", "string", false, &["Input", "Output"]),
    ("A_ARG_TYPE_ProtocolInfo", "string", false, &[]),
    ("A_ARG_TYPE_ConnectionID", "i4", false, &[]),
    ("A_ARG_TYPE_AVTransportID", "i4", false, &[]),
    ("A_ARG_TYPE_RcsID", "i4", false, &[]),
];

pub fn scpd(svc: Service) -> String {
    let (acts, vars) = match svc {
        Service::Avt => (AVT_ACTIONS, AVT_VARS),
        Service::Rc => (RC_ACTIONS, RC_VARS),
        Service::Cm => (CM_ACTIONS, CM_VARS),
    };
    let mut s = String::from("<?xml version=\"1.0\"?>\r\n<scpd xmlns=\"urn:schemas-upnp-org:service-1-0\"><specVersion><major>1</major><minor>0</minor></specVersion><actionList>");
    for (name, args) in acts {
        s.push_str(&format!("<action><name>{name}</name><argumentList>"));
        for (an, dir, rel) in *args {
            s.push_str(&format!("<argument><name>{an}</name><direction>{dir}</direction><relatedStateVariable>{rel}</relatedStateVariable></argument>"));
        }
        s.push_str("</argumentList></action>");
    }
    s.push_str("</actionList><serviceStateTable>");
    for (name, ty, ev, allowed) in vars {
        s.push_str(&format!("<stateVariable sendEvents=\"{}\"><name>{name}</name><dataType>{ty}</dataType>", if *ev { "yes" } else { "no" }));
        if !allowed.is_empty() {
            s.push_str("<allowedValueList>");
            for a in *allowed {
                s.push_str(&format!("<allowedValue>{a}</allowedValue>"));
            }
            s.push_str("</allowedValueList>");
        }
        if *name == "Volume" {
            s.push_str("<allowedValueRange><minimum>0</minimum><maximum>100</maximum><step>1</step></allowedValueRange>");
        }
        s.push_str("</stateVariable>");
    }
    s.push_str("</serviceStateTable></scpd>");
    s
}

const SINK: &str = "http-get:*:video/mp4:*,http-get:*:video/x-matroska:*,http-get:*:video/webm:*,http-get:*:video/mpeg:*,http-get:*:video/x-msvideo:*,http-get:*:video/quicktime:*,http-get:*:video/x-ms-wmv:*,http-get:*:video/vnd.dlna.mpeg-tts:*,http-get:*:audio/mpeg:*,http-get:*:audio/mp4:*,http-get:*:audio/aac:*,http-get:*:audio/flac:*,http-get:*:audio/x-flac:*,http-get:*:audio/ogg:*,http-get:*:audio/wav:*,http-get:*:audio/x-wav:*,http-get:*:audio/L16:*,http-get:*:application/vnd.apple.mpegurl:*,http-get:*:application/x-mpegURL:*";

// ---------- état du rendu et actions ----------

#[derive(Default)]
pub struct Renderer {
    pub uri: String,
    pub meta: String,
    pub title: String,
    pub kind: String,
    /// Vrai si le lecteur joue le média poussé par un téléphone (et pas autre chose lancé depuis RusTV).
    pub ours: bool,
    /// Volume avant le « muet » simulé (SetMute).
    pub premute: Option<u8>,
}

type Out = Vec<(String, String)>;
type Fault = (u16, &'static str);

fn arg<'a>(args: &'a [(String, String)], k: &str) -> Option<&'a str> {
    args.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
}

fn o(pairs: &[(&str, &str)]) -> Out {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

impl Renderer {
    /// Met à jour `ours` d'après l'état réel du lecteur et renvoie l'état de transport UPnP.
    pub fn transport_state(&mut self, snap: &PlayerSnap) -> &'static str {
        if !snap.active {
            self.ours = false;
        }
        if self.uri.is_empty() {
            "NO_MEDIA_PRESENT"
        } else if self.ours && snap.paused {
            "PAUSED_PLAYBACK"
        } else if self.ours {
            "PLAYING"
        } else {
            "STOPPED"
        }
    }

    fn actions(state: &str) -> &'static str {
        match state {
            "PLAYING" => "Pause,Stop,Seek",
            "PAUSED_PLAYBACK" => "Play,Stop,Seek",
            "STOPPED" => "Play,Stop",
            _ => "",
        }
    }

    fn check_instance(args: &[(String, String)]) -> Result<(), Fault> {
        match arg(args, "InstanceID") {
            Some(v) if v.trim() == "0" => Ok(()),
            Some(_) => Err((718, "Invalid InstanceID")),
            None => Err((402, "Invalid Args")),
        }
    }

    pub fn avt(&mut self, host: &dyn Host, action: &str, args: &[(String, String)]) -> Result<Out, Fault> {
        Self::check_instance(args)?;
        let snap = host.player();
        let state = self.transport_state(&snap);
        let pos = if self.ours { snap.position } else { 0.0 };
        let dur = if self.ours { snap.duration } else { 0.0 };
        match action {
            "SetAVTransportURI" => {
                let raw = arg(args, "CurrentURI").ok_or((402, "Invalid Args"))?;
                if self.ours {
                    host.stop();
                }
                self.ours = false;
                if raw.trim().is_empty() {
                    *self = Renderer { premute: self.premute, ..Default::default() };
                    return Ok(vec![]);
                }
                let url = validate_media_url(raw).map_err(|_| (716, "Resource not found"))?;
                let meta = arg(args, "CurrentURIMetaData").unwrap_or("");
                let d = parse_didl(meta);
                let kind = classify(&d, &url);
                if kind == "image" {
                    return Err((714, "Illegal MIME-type"));
                }
                self.title = if d.title.is_empty() { title_from_url(&url) } else { d.title.clone() };
                self.kind = kind.to_string();
                self.uri = url;
                self.meta = meta.chars().take(MAX_ARG).collect();
                Ok(vec![])
            }
            "Play" => {
                if self.uri.is_empty() {
                    return Err((701, "Transition not available"));
                }
                if self.ours && snap.active {
                    if snap.paused {
                        host.pause(false);
                    }
                } else {
                    host.play(&self.kind, &self.title, &self.uri).map_err(|_| (501, "Action Failed"))?;
                    self.ours = true;
                }
                Ok(vec![])
            }
            "Pause" => {
                if !(self.ours && snap.active) {
                    return Err((701, "Transition not available"));
                }
                host.pause(true);
                Ok(vec![])
            }
            "Stop" => {
                if self.ours {
                    host.stop();
                    self.ours = false;
                }
                Ok(vec![])
            }
            "Seek" => {
                let unit = arg(args, "Unit").ok_or((402, "Invalid Args"))?;
                let target = arg(args, "Target").ok_or((402, "Invalid Args"))?;
                match unit {
                    "REL_TIME" | "ABS_TIME" => {
                        let t = parse_reltime(target).ok_or((711, "Illegal seek target"))?;
                        if !(self.ours && snap.active) {
                            return Err((701, "Transition not available"));
                        }
                        host.seek(t.max(0.0));
                        Ok(vec![])
                    }
                    "TRACK_NR" if target.trim() == "1" || target.trim() == "0" => Ok(vec![]),
                    _ => Err((710, "Seek mode not supported")),
                }
            }
            // Une seule piste : sans effet.
            "Next" | "Previous" => Ok(vec![]),
            "GetTransportInfo" => Ok(o(&[("CurrentTransportState", state), ("CurrentTransportStatus", "OK"), ("CurrentSpeed", "1")])),
            "GetPositionInfo" => {
                let has = !self.uri.is_empty();
                Ok(o(&[
                    ("Track", if has { "1" } else { "0" }),
                    ("TrackDuration", &fmt_reltime(dur)),
                    ("TrackMetaData", &self.meta),
                    ("TrackURI", &self.uri),
                    ("RelTime", &fmt_reltime(pos)),
                    ("AbsTime", &fmt_reltime(pos)),
                    ("RelCount", "2147483647"),
                    ("AbsCount", "2147483647"),
                ]))
            }
            "GetMediaInfo" => Ok(o(&[
                ("NrTracks", if self.uri.is_empty() { "0" } else { "1" }),
                ("MediaDuration", &fmt_reltime(dur)),
                ("CurrentURI", &self.uri),
                ("CurrentURIMetaData", &self.meta),
                ("NextURI", ""),
                ("NextURIMetaData", ""),
                ("PlayMedium", "NETWORK"),
                ("RecordMedium", "NOT_IMPLEMENTED"),
                ("WriteStatus", "NOT_IMPLEMENTED"),
            ])),
            "GetCurrentTransportActions" => Ok(o(&[("Actions", Self::actions(state))])),
            "GetDeviceCapabilities" => Ok(o(&[("PlayMedia", "NETWORK"), ("RecMedia", "NOT_IMPLEMENTED"), ("RecQualityModes", "NOT_IMPLEMENTED")])),
            "GetTransportSettings" => Ok(o(&[("PlayMode", "NORMAL"), ("RecQualityMode", "NOT_IMPLEMENTED")])),
            _ => Err((401, "Invalid Action")),
        }
    }

    pub fn rc(&mut self, host: &dyn Host, action: &str, args: &[(String, String)]) -> Result<Out, Fault> {
        Self::check_instance(args)?;
        let channel_ok = || matches!(arg(args, "Channel"), Some("Master") | None);
        match action {
            "GetVolume" => {
                if !channel_ok() {
                    return Err((402, "Invalid Args"));
                }
                Ok(o(&[("CurrentVolume", &self.volume(host).0.to_string())]))
            }
            "SetVolume" => {
                if !channel_ok() {
                    return Err((402, "Invalid Args"));
                }
                let v: u8 = arg(args, "DesiredVolume").and_then(|v| v.trim().parse::<u16>().ok()).filter(|v| *v <= 100).ok_or((402, "Invalid Args"))? as u8;
                self.premute = None;
                host.set_volume(v);
                Ok(vec![])
            }
            "GetMute" => Ok(o(&[("CurrentMute", if self.volume(host).1 { "1" } else { "0" })])),
            "SetMute" => {
                let m = match arg(args, "DesiredMute").map(str::trim) {
                    Some("1") | Some("true") | Some("True") => true,
                    Some("0") | Some("false") | Some("False") => false,
                    _ => return Err((402, "Invalid Args")),
                };
                // Pas d'API « muet » dans rustv-core : on met le volume à 0 et on le rétablit ensuite.
                if m && self.premute.is_none() {
                    let (v, _) = host.volume();
                    self.premute = Some(v);
                    host.set_volume(0);
                } else if !m {
                    if let Some(v) = self.premute.take() {
                        host.set_volume(if v == 0 { 20 } else { v });
                    }
                }
                Ok(vec![])
            }
            "ListPresets" => Ok(o(&[("CurrentPresetNameList", "FactoryDefaults")])),
            "SelectPreset" => Ok(vec![]),
            _ => Err((401, "Invalid Action")),
        }
    }

    /// (volume, muet) tels que vus par le contrôleur.
    pub fn volume(&self, host: &dyn Host) -> (u8, bool) {
        let (v, m) = host.volume();
        match self.premute {
            Some(p) => (p, true),
            None => (v, m),
        }
    }
}

pub fn cm(action: &str, args: &[(String, String)]) -> Result<Out, Fault> {
    match action {
        "GetProtocolInfo" => Ok(o(&[("Source", ""), ("Sink", SINK)])),
        "GetCurrentConnectionIDs" => Ok(o(&[("ConnectionIDs", "0")])),
        "GetCurrentConnectionInfo" => {
            if arg(args, "ConnectionID").map(str::trim) != Some("0") {
                return Err((706, "Invalid connection reference"));
            }
            Ok(o(&[("RcsID", "0"), ("AVTransportID", "0"), ("ProtocolInfo", ""), ("PeerConnectionManager", ""), ("PeerConnectionID", "-1"), ("Direction", "Input"), ("Status", "OK")]))
        }
        _ => Err((401, "Invalid Action")),
    }
}

/// Traite une requête de contrôle SOAP et construit la réponse HTTP.
pub fn control(sh: &Shared, svc: Service, req: &Request) -> Response {
    let fault = |c: u16, d: &str| Response::xml(500, soap_fault(c, d));
    if req.method != "POST" {
        return Response::text(405, "Method not allowed");
    }
    let Ok(soap) = parse_soap(&req.body_str()) else { return Response::text(400, "Bad SOAP") };
    // L'en-tête SOAPACTION doit, s'il est présent, désigner la même action.
    if let Some(h) = req.header("soapaction") {
        let a = h.trim().trim_matches('"');
        if let Some((_, name)) = a.rsplit_once('#') {
            if name != soap.action {
                return fault(401, "Invalid Action");
            }
        }
    }
    let res = {
        let mut r = sh.renderer.lock().unwrap();
        match svc {
            Service::Avt => r.avt(sh.host.as_ref(), &soap.action, &soap.args),
            Service::Rc => r.rc(sh.host.as_ref(), &soap.action, &soap.args),
            Service::Cm => cm(&soap.action, &soap.args),
        }
    };
    {
        let mut st = sh.status.lock().unwrap();
        st.last_event = soap.action.clone();
    }
    match res {
        Ok(out) => Response::xml(200, soap_response(svc, &soap.action, &out)).header("EXT", ""),
        Err((c, d)) => fault(c, d),
    }
}

// ---------- GENA (évènements) ----------

pub struct Sub {
    pub sid: String,
    pub service: Service,
    pub callback: String,
    pub expires: Instant,
    pub seq: u32,
    pub initial_sent: bool,
    pub fails: u8,
}

/// Extrait la première URL `<http://...>` de l'en-tête CALLBACK ; elle doit pointer vers l'abonné lui-même.
pub fn parse_callback(h: &str, peer: IpAddr) -> Option<String> {
    let start = h.find('<')?;
    let end = h[start..].find('>')? + start;
    let u = h[start + 1..end].trim();
    if u.len() > 512 || u.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return None;
    }
    let p: tauri::Url = u.parse().ok()?;
    if p.scheme() != "http" || !p.username().is_empty() || p.port_or_known_default().is_none_or(|x| x == 0) {
        return None;
    }
    let host: IpAddr = p.host_str()?.parse().ok()?;
    // Le rappel doit viser l'appareil qui s'abonne : pas de requêtes vers un tiers (SSRF).
    (host == peer && super::http::is_lan(host)).then(|| p.to_string())
}

/// `Second-NNN` → secondes bornées à 60..=1800 ; « infinite » ou absent → 1800.
pub fn parse_timeout(h: Option<&str>) -> u64 {
    h.and_then(|v| v.trim().to_ascii_lowercase().strip_prefix("second-").and_then(|n| n.parse::<u64>().ok())).unwrap_or(1800).clamp(60, 1800)
}

fn new_sid() -> String {
    format!("uuid:{}", super::new_uuid())
}

pub fn subscribe(sh: &Shared, svc: Service, req: &Request) -> Response {
    let ok = |sid: &str, t: u64| Response::new(200).header("SID", sid).header("TIMEOUT", format!("Second-{t}")).header("Server", sh.server.clone());
    let timeout = parse_timeout(req.header("timeout"));
    let mut subs = sh.subs.lock().unwrap();
    subs.retain(|s| s.expires > Instant::now());
    // Renouvellement.
    if let Some(sid) = req.header("sid") {
        if req.header("callback").is_some() || req.header("nt").is_some() {
            return Response::text(400, "Bad request");
        }
        return match subs.iter_mut().find(|s| s.sid == sid && s.service == svc) {
            Some(s) => {
                s.expires = Instant::now() + Duration::from_secs(timeout);
                ok(sid, timeout)
            }
            None => Response::text(412, "Precondition failed"),
        };
    }
    if req.header("nt").map(str::trim) != Some("upnp:event") {
        return Response::text(412, "Precondition failed");
    }
    let Some(cb) = req.header("callback").and_then(|c| parse_callback(c, req.peer.ip())) else { return Response::text(412, "Precondition failed") };
    let mut recent = sh.sub_times.lock().unwrap();
    recent.retain(|t| t.elapsed() < Duration::from_secs(60));
    if recent.len() >= MAX_NEW_SUBS_PER_MIN {
        return Response::text(429, "Too many subscriptions");
    }
    if subs.len() >= MAX_SUBS {
        return Response::text(503, "Too many subscribers");
    }
    recent.push(Instant::now());
    let sid = new_sid();
    subs.push(Sub { sid: sid.clone(), service: svc, callback: cb, expires: Instant::now() + Duration::from_secs(timeout), seq: 0, initial_sent: false, fails: 0 });
    sh.status.lock().unwrap().last_client = req.peer.ip().to_string();
    ok(&sid, timeout)
}

pub fn unsubscribe(sh: &Shared, svc: Service, req: &Request) -> Response {
    let Some(sid) = req.header("sid") else { return Response::text(412, "Precondition failed") };
    let mut subs = sh.subs.lock().unwrap();
    let before = subs.len();
    subs.retain(|s| !(s.sid == sid && s.service == svc));
    if subs.len() < before {
        Response::new(200)
    } else {
        Response::text(412, "Precondition failed")
    }
}

pub fn propertyset(props: &[(&str, String)]) -> String {
    let mut s = String::from("<?xml version=\"1.0\"?>\r\n<e:propertyset xmlns:e=\"urn:schemas-upnp-org:event-1-0\">");
    for (k, v) in props {
        s.push_str(&format!("<e:property><{k}>{}</{k}></e:property>", xml_escape(v)));
    }
    s.push_str("</e:propertyset>");
    s
}

#[derive(PartialEq, Clone)]
pub struct Snap {
    pub transport: String,
    pub uri: String,
    pub volume: u8,
    pub mute: bool,
}

pub fn avt_last_change(s: &Snap) -> String {
    format!(
        "<Event xmlns=\"urn:schemas-upnp-org:metadata-1-0/AVT/\"><InstanceID val=\"0\"><TransportState val=\"{}\"/><TransportStatus val=\"OK\"/><CurrentTrackURI val=\"{}\"/><AVTransportURI val=\"{}\"/><CurrentTransportActions val=\"{}\"/></InstanceID></Event>",
        s.transport,
        xml_escape(&s.uri),
        xml_escape(&s.uri),
        Renderer::actions(&s.transport)
    )
}

pub fn rc_last_change(s: &Snap) -> String {
    format!(
        "<Event xmlns=\"urn:schemas-upnp-org:metadata-1-0/RCS/\"><InstanceID val=\"0\"><Volume channel=\"Master\" val=\"{}\"/><Mute channel=\"Master\" val=\"{}\"/></InstanceID></Event>",
        s.volume,
        u8::from(s.mute)
    )
}

fn event_body(svc: Service, s: &Snap) -> String {
    match svc {
        Service::Avt => propertyset(&[("LastChange", avt_last_change(s))]),
        Service::Rc => propertyset(&[("LastChange", rc_last_change(s))]),
        Service::Cm => propertyset(&[("SourceProtocolInfo", String::new()), ("SinkProtocolInfo", SINK.to_string()), ("CurrentConnectionIDs", "0".into())]),
    }
}

fn snapshot(sh: &Shared) -> Snap {
    let snap = sh.host.player();
    let mut r = sh.renderer.lock().unwrap();
    let transport = r.transport_state(&snap).to_string();
    let (volume, mute) = r.volume(sh.host.as_ref());
    Snap { transport, uri: r.uri.clone(), volume, mute }
}

/// Envoie un NOTIFY GENA. Vrai si le récepteur a répondu (même par une erreur HTTP).
fn send_notice(agent: &ureq::Agent, cb: &str, sid: &str, seq: u32, body: &str) -> bool {
    match agent
        .request("NOTIFY", cb)
        .set("Content-Type", "text/xml; charset=\"utf-8\"")
        .set("NT", "upnp:event")
        .set("NTS", "upnp:propchange")
        .set("SID", sid)
        .set("SEQ", &seq.to_string())
        .send_string(body)
    {
        Ok(_) | Err(ureq::Error::Status(..)) => true,
        Err(_) => false,
    }
}

/// Fil d'évènements : détecte les changements (lecture, volume) et notifie les abonnés ; purge les expirés.
pub fn run_gena(sh: Arc<Shared>) {
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(Duration::from_secs(1))
        .timeout_read(Duration::from_secs(2))
        .timeout_write(Duration::from_secs(2))
        .user_agent("RusTV-cast")
        .build();
    let mut last: Option<Snap> = None;
    while !sh.stop.load(Ordering::Relaxed) {
        for _ in 0..10 {
            if sh.stop.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        {
            let mut subs = sh.subs.lock().unwrap();
            subs.retain(|s| s.expires > Instant::now());
            if subs.is_empty() {
                last = None;
                continue;
            }
        }
        let snap = snapshot(&sh);
        let changed = last.as_ref() != Some(&snap);
        // Prépare les envois sous verrou, les exécute sans verrou.
        let mut jobs = Vec::new();
        for s in sh.subs.lock().unwrap().iter_mut() {
            if !s.initial_sent || (changed && s.service != Service::Cm) {
                jobs.push((s.sid.clone(), s.service, s.callback.clone(), s.seq));
                s.seq = s.seq.wrapping_add(1).max(1);
                s.initial_sent = true;
            }
        }
        last = Some(snap.clone());
        for (sid, svc, cb, seq) in jobs {
            let ok = send_notice(&agent, &cb, &sid, seq, &event_body(svc, &snap));
            let mut subs = sh.subs.lock().unwrap();
            if let Some(s) = subs.iter_mut().find(|s| s.sid == sid) {
                s.fails = if ok { 0 } else { s.fails + 1 };
            }
            // Un abonné injoignable deux fois de suite est retiré.
            subs.retain(|s| s.fails < 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testutil::MockHost;
    use super::*;

    fn a(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn reltime_format_parse() {
        assert_eq!(fmt_reltime(0.0), "0:00:00");
        assert_eq!(fmt_reltime(3661.9), "1:01:01");
        assert_eq!(fmt_reltime(-5.0), "0:00:00");
        assert_eq!(fmt_reltime(f64::NAN), "0:00:00");
        assert_eq!(parse_reltime("0:01:30"), Some(90.0));
        assert_eq!(parse_reltime("01:02:03.5"), Some(3723.5));
        assert_eq!(parse_reltime("+1:00:00"), Some(3600.0));
        assert_eq!(parse_reltime("NOT_IMPLEMENTED"), None);
        assert_eq!(parse_reltime("0:61:00"), None);
        assert_eq!(parse_reltime("0:1:00"), None);
        assert_eq!(parse_reltime("1:00"), None);
        assert_eq!(parse_reltime("a:00:00"), None);
    }

    #[test]
    fn url_validation() {
        assert!(validate_media_url("http://192.168.1.5:8200/a.mp4").is_ok());
        assert!(validate_media_url("https://example.com/v?x=1").is_ok());
        for bad in ["file:///C:/x.mp4", "C:\\x.mp4", "\\\\host\\share\\a.mp4", "/etc/passwd", "ftp://a/b", "javascript:alert(1)", "http://u:p@host/a", "http://", "", "http://a/b c", "rtsp://a/b"] {
            assert!(validate_media_url(bad).is_err(), "{bad}");
        }
        assert!(validate_media_url(&format!("http://a/{}", "x".repeat(3000))).is_err());
    }

    const DIDL: &str = r#"<DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/"><item id="1" parentID="0" restricted="1"><dc:title>Film &amp; Co</dc:title><upnp:class>object.item.videoItem.movie</upnp:class><upnp:albumArtURI>http://192.168.1.5/art.jpg</upnp:albumArtURI><res protocolInfo="http-get:*:video/mp4:DLNA.ORG_PN=AVC">http://192.168.1.5/a.mp4</res></item></DIDL-Lite>"#;

    #[test]
    fn didl() {
        let d = parse_didl(DIDL);
        assert_eq!(d.title, "Film & Co");
        assert_eq!(d.class, "object.item.videoItem.movie");
        assert_eq!(d.album_art, "http://192.168.1.5/art.jpg");
        assert_eq!(d.mime, "video/mp4");
        assert_eq!(classify(&d, "http://x/a"), "video");
        assert_eq!(parse_didl(""), Didl::default());
        assert_eq!(parse_didl("<!DOCTYPE x [<!ENTITY a \"b\">]><DIDL-Lite/>"), Didl::default());
        // Jaquette non http(s) ignorée.
        assert_eq!(parse_didl("<d><upnp:albumArtURI>file:///x.jpg</upnp:albumArtURI></d>").album_art, "");
    }

    #[test]
    fn classification() {
        let d = |class: &str, mime: &str| Didl { class: class.into(), mime: mime.into(), ..Default::default() };
        assert_eq!(classify(&d("object.item.audioItem.musicTrack", ""), "http://a/x"), "audio");
        assert_eq!(classify(&d("object.item.imageItem.photo", ""), "http://a/x"), "image");
        assert_eq!(classify(&d("", "audio/flac"), "http://a/x"), "audio");
        assert_eq!(classify(&d("", ""), "http://a/x.MP3?t=1"), "audio");
        assert_eq!(classify(&d("", ""), "http://a/x.jpg"), "image");
        assert_eq!(classify(&d("", ""), "http://a/stream"), "video");
    }

    const ENV: &str = r#"<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/"><s:Body><u:SetAVTransportURI xmlns:u="urn:schemas-upnp-org:service:AVTransport:1"><InstanceID>0</InstanceID><CurrentURI>http://a/b?x=1&amp;y=2</CurrentURI><CurrentURIMetaData>&lt;DIDL-Lite&gt;&lt;/DIDL-Lite&gt;</CurrentURIMetaData></u:SetAVTransportURI></s:Body></s:Envelope>"#;

    #[test]
    fn soap_parse() {
        let s = parse_soap(ENV).unwrap();
        assert_eq!(s.action, "SetAVTransportURI");
        assert_eq!(arg(&s.args, "CurrentURI"), Some("http://a/b?x=1&y=2"));
        assert_eq!(arg(&s.args, "CurrentURIMetaData"), Some("<DIDL-Lite></DIDL-Lite>"));
        let stop = parse_soap(r#"<s:Envelope xmlns:s="x"><s:Body><u:Stop xmlns:u="y"><InstanceID>0</InstanceID></u:Stop></s:Body></s:Envelope>"#).unwrap();
        assert_eq!((stop.action.as_str(), stop.args.len()), ("Stop", 1));
        let empty = parse_soap(r#"<s:Envelope xmlns:s="x"><s:Body><u:Next xmlns:u="y"/></s:Body></s:Envelope>"#).unwrap();
        assert_eq!(empty.action, "Next");
    }

    #[test]
    fn soap_reject() {
        assert!(parse_soap("").is_err());
        assert!(parse_soap("<a/>").is_err());
        assert!(parse_soap("<!DOCTYPE s [<!ENTITY x \"y\">]><s:Envelope><s:Body><u:Stop/></s:Body></s:Envelope>").is_err());
        assert!(parse_soap("<s:Envelope><s:Body><u:Stop><a><b><c/></b></a></u:Stop></s:Body></s:Envelope>").is_err());
        let many: String = (0..40).map(|i| format!("<a{i}>1</a{i}>")).collect();
        assert!(parse_soap(&format!("<s:Envelope><s:Body><u:X>{many}</u:X></s:Body></s:Envelope>")).is_err());
    }

    #[test]
    fn soap_response_and_fault() {
        let r = soap_response(Service::Avt, "GetTransportInfo", &[("CurrentTransportState".into(), "PLAYING".into()), ("X".into(), "a<b".into())]);
        assert!(r.contains("<u:GetTransportInfoResponse xmlns:u=\"urn:schemas-upnp-org:service:AVTransport:1\">"));
        assert!(r.contains("<CurrentTransportState>PLAYING</CurrentTransportState>") && r.contains("<X>a&lt;b</X>"));
        let f = soap_fault(701, "Transition not available");
        assert!(f.contains("<errorCode>701</errorCode>") && f.contains("UPnPError"));
        // Le résultat doit être du XML valide pour notre propre lecteur.
        assert!(parse_soap(&r).is_ok());
    }

    #[test]
    fn scpd_lists_actions() {
        let a = scpd(Service::Avt);
        for n in ["SetAVTransportURI", "Play", "Pause", "Stop", "Seek", "Next", "Previous", "GetTransportInfo", "GetPositionInfo", "GetMediaInfo", "GetCurrentTransportActions"] {
            assert!(a.contains(&format!("<name>{n}</name>")), "{n}");
        }
        assert!(a.contains("sendEvents=\"yes\"><name>LastChange"));
        let r = scpd(Service::Rc);
        assert!(r.contains("<name>SetVolume</name>") && r.contains("<maximum>100</maximum>"));
        assert!(scpd(Service::Cm).contains("GetProtocolInfo"));
        let d = device_xml("RusTV (PC<1>)", "uuid-x");
        assert!(d.contains("<friendlyName>RusTV (PC&lt;1&gt;)</friendlyName>") && d.contains("<UDN>uuid:uuid-x</UDN>"));
        assert!(d.contains("MediaRenderer:1") && d.contains("/dlna/avt/control") && d.contains("/dlna/rc/event"));
    }

    #[test]
    fn gena_headers() {
        let peer: IpAddr = "192.168.1.30".parse().unwrap();
        assert_eq!(parse_callback("<http://192.168.1.30:49152/evt>", peer).as_deref(), Some("http://192.168.1.30:49152/evt"));
        assert!(parse_callback("<http://192.168.1.31:49152/evt>", peer).is_none(), "autre hôte");
        assert!(parse_callback("<http://8.8.8.8/evt>", "8.8.8.8".parse().unwrap()).is_none(), "public");
        assert!(parse_callback("<https://192.168.1.30/evt>", peer).is_none());
        assert!(parse_callback("http://192.168.1.30/evt", peer).is_none());
        assert!(parse_callback("<http://evil.local/evt>", peer).is_none());
        assert_eq!(parse_timeout(Some("Second-300")), 300);
        assert_eq!(parse_timeout(Some("Second-99999")), 1800);
        assert_eq!(parse_timeout(Some("Second-1")), 60);
        assert_eq!(parse_timeout(Some("infinite")), 1800);
        assert_eq!(parse_timeout(None), 1800);
    }

    #[test]
    fn last_change() {
        let s = Snap { transport: "PLAYING".into(), uri: "http://a/b?x=1&y=2".into(), volume: 30, mute: false };
        assert!(avt_last_change(&s).contains("<TransportState val=\"PLAYING\"/>") && avt_last_change(&s).contains("x=1&amp;y=2"));
        assert!(rc_last_change(&s).contains("<Volume channel=\"Master\" val=\"30\"/>"));
        let body = event_body(Service::Rc, &s);
        // La valeur de LastChange est échappée une fois dans le propertyset.
        assert!(body.contains("&lt;Event") && body.contains("<e:propertyset"));
    }

    #[test]
    fn avt_flow() {
        let h = MockHost::default();
        let mut r = Renderer::default();
        let uri = [("InstanceID", "0"), ("CurrentURI", "http://192.168.1.5/m.mp4"), ("CurrentURIMetaData", DIDL)];
        assert!(r.avt(&h, "Play", &a(&[("InstanceID", "0")])).is_err(), "pas d'URI");
        assert_eq!(r.avt(&h, "GetTransportInfo", &a(&[("InstanceID", "0")])).unwrap()[0].1, "NO_MEDIA_PRESENT");
        r.avt(&h, "SetAVTransportURI", &a(&uri)).unwrap();
        assert_eq!(r.title, "Film & Co");
        assert_eq!(r.avt(&h, "GetTransportInfo", &a(&[("InstanceID", "0")])).unwrap()[0].1, "STOPPED");
        r.avt(&h, "Play", &a(&[("InstanceID", "0"), ("Speed", "1")])).unwrap();
        assert_eq!(h.log(), vec!["play video Film & Co http://192.168.1.5/m.mp4"]);
        assert_eq!(r.avt(&h, "GetTransportInfo", &a(&[("InstanceID", "0")])).unwrap()[0].1, "PLAYING");
        r.avt(&h, "Seek", &a(&[("InstanceID", "0"), ("Unit", "REL_TIME"), ("Target", "0:01:30")])).unwrap();
        r.avt(&h, "Pause", &a(&[("InstanceID", "0")])).unwrap();
        assert_eq!(r.avt(&h, "GetTransportInfo", &a(&[("InstanceID", "0")])).unwrap()[0].1, "PAUSED_PLAYBACK");
        r.avt(&h, "Play", &a(&[("InstanceID", "0")])).unwrap();
        let pos = r.avt(&h, "GetPositionInfo", &a(&[("InstanceID", "0")])).unwrap();
        assert_eq!(pos[1].1, "0:10:00");
        assert!(r.avt(&h, "Seek", &a(&[("InstanceID", "0"), ("Unit", "REL_TIME"), ("Target", "garbage")])).is_err());
        r.avt(&h, "Stop", &a(&[("InstanceID", "0")])).unwrap();
        assert_eq!(h.log()[1..], ["seek 90", "pause true", "pause false", "stop"]);
        assert_eq!(r.avt(&h, "GetCurrentTransportActions", &a(&[("InstanceID", "0")])).unwrap()[0].1, "Play,Stop");
        assert_eq!(r.avt(&h, "Foo", &a(&[("InstanceID", "0")])).unwrap_err().0, 401);
        assert_eq!(r.avt(&h, "Stop", &a(&[("InstanceID", "5")])).unwrap_err().0, 718);
    }

    #[test]
    fn avt_rejects() {
        let h = MockHost::default();
        let mut r = Renderer::default();
        for bad in ["file:///C:/a.mp4", "C:\\a.mp4"] {
            let e = r.avt(&h, "SetAVTransportURI", &a(&[("InstanceID", "0"), ("CurrentURI", bad), ("CurrentURIMetaData", "")])).unwrap_err();
            assert_eq!(e.0, 716);
        }
        let img = r.avt(&h, "SetAVTransportURI", &a(&[("InstanceID", "0"), ("CurrentURI", "http://a/p.jpg"), ("CurrentURIMetaData", "")])).unwrap_err();
        assert_eq!(img.0, 714);
        assert!(r.uri.is_empty() && h.log().is_empty());
    }

    #[test]
    fn rc_volume_mute() {
        let h = MockHost::default();
        let mut r = Renderer::default();
        assert_eq!(r.rc(&h, "GetVolume", &a(&[("InstanceID", "0"), ("Channel", "Master")])).unwrap()[0].1, "40");
        r.rc(&h, "SetVolume", &a(&[("InstanceID", "0"), ("Channel", "Master"), ("DesiredVolume", "55")])).unwrap();
        assert!(r.rc(&h, "SetVolume", &a(&[("InstanceID", "0"), ("Channel", "Master"), ("DesiredVolume", "101")])).is_err());
        assert!(r.rc(&h, "SetVolume", &a(&[("InstanceID", "0"), ("Channel", "Master"), ("DesiredVolume", "-1")])).is_err());
        assert!(r.rc(&h, "GetVolume", &a(&[("InstanceID", "0"), ("Channel", "LF")])).is_err());
        r.rc(&h, "SetMute", &a(&[("InstanceID", "0"), ("Channel", "Master"), ("DesiredMute", "1")])).unwrap();
        assert_eq!(r.rc(&h, "GetMute", &a(&[("InstanceID", "0")])).unwrap()[0].1, "1");
        r.rc(&h, "SetMute", &a(&[("InstanceID", "0"), ("Channel", "Master"), ("DesiredMute", "0")])).unwrap();
        assert_eq!(h.log(), vec!["volume 55", "volume 0", "volume 55"]);
    }

    #[test]
    fn connection_manager() {
        assert!(cm("GetProtocolInfo", &[]).unwrap()[1].1.contains("video/mp4"));
        assert_eq!(cm("GetCurrentConnectionIDs", &[]).unwrap()[0].1, "0");
        assert!(cm("GetCurrentConnectionInfo", &a(&[("ConnectionID", "7")])).is_err());
    }
}
