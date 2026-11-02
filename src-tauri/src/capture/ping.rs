use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, Socket, Type};

use crate::events::NetPing;

const HOST: &str = "www.google.com";
const PORT: u16 = 443;
const PROBE_INTERVAL: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(1500);
const MAX_ATTEMPTS: usize = 3;

#[derive(Clone, Default)]
pub struct PingSampler {
    last: Arc<Mutex<Vec<NetPing>>>,
}

impl PingSampler {
    pub fn new() -> Self {
        let last: Arc<Mutex<Vec<NetPing>>> = Arc::new(Mutex::new(Vec::new()));
        let shared = Arc::clone(&last);
        thread::spawn(move || {
            let mut addrs: Vec<SocketAddr> = Vec::new();
            loop {
                if addrs.is_empty() {
                    addrs = resolve();
                }
                let measured = probe_cycle(&mut addrs);
                if let Ok(mut slot) = shared.lock() {
                    *slot = measured;
                }
                thread::sleep(PROBE_INTERVAL);
            }
        });
        Self { last }
    }

    pub fn sample(&self) -> Vec<NetPing> {
        self.last
            .lock()
            .map(|slot| slot.clone())
            .unwrap_or_default()
    }
}

fn probe_cycle(addrs: &mut Vec<SocketAddr>) -> Vec<NetPing> {
    if addrs.is_empty() {
        return Vec::new();
    }
    #[cfg(windows)]
    {
        windows_probe(addrs)
    }
    #[cfg(not(windows))]
    {
        legacy_probe(addrs)
    }
}

#[cfg(windows)]
fn windows_probe(addrs: &mut Vec<SocketAddr>) -> Vec<NetPing> {
    let interfaces = local_interfaces();
    if interfaces.is_empty() {
        return legacy_probe(addrs);
    }
    let v4: Vec<SocketAddr> = addrs.iter().filter(|a| a.is_ipv4()).copied().collect();
    if v4.is_empty() {
        return legacy_probe(addrs);
    }
    let mut out = Vec::new();
    let mut any_success = false;
    for (name, source) in interfaces {
        if let Some(ms) = probe_interface(source, &v4) {
            any_success = true;
            out.push(NetPing { name, ms });
        }
    }
    if !any_success {
        addrs.clear();
    }
    out
}

fn legacy_probe(addrs: &mut Vec<SocketAddr>) -> Vec<NetPing> {
    for &addr in addrs.iter().take(MAX_ATTEMPTS) {
        let start = Instant::now();
        if TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).is_ok() {
            return vec![NetPing {
                name: String::new(),
                ms: start.elapsed().as_secs_f64() * 1000.0,
            }];
        }
    }
    addrs.clear();
    Vec::new()
}

#[cfg(windows)]
fn probe_interface(source: Ipv4Addr, targets: &[SocketAddr]) -> Option<f64> {
    for target in targets.iter().take(MAX_ATTEMPTS) {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).ok()?;
        let local = SocketAddr::new(IpAddr::V4(source), 0);
        socket.bind(&local.into()).ok()?;
        let start = Instant::now();
        if socket
            .connect_timeout(&(*target).into(), CONNECT_TIMEOUT)
            .is_ok()
        {
            return Some(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    None
}

#[cfg(windows)]
fn local_interfaces() -> Vec<(String, Ipv4Addr)> {
    use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR};
    use windows::Win32::NetworkManagement::IpHelper::{
        GET_ADAPTERS_ADDRESSES_FLAGS, GetAdaptersAddresses, IF_TYPE_SOFTWARE_LOOPBACK,
        IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;

    let mut out = Vec::new();
    unsafe {
        let mut size: u32 = 0;
        if GetAdaptersAddresses(
            windows::Win32::Networking::WinSock::AF_INET.0 as u32,
            GET_ADAPTERS_ADDRESSES_FLAGS(0),
            None,
            None,
            &mut size,
        ) != ERROR_BUFFER_OVERFLOW.0
        {
            return out;
        }
        let mut buffer = vec![0u8; size as usize];
        let head = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        if GetAdaptersAddresses(
            windows::Win32::Networking::WinSock::AF_INET.0 as u32,
            GET_ADAPTERS_ADDRESSES_FLAGS(0),
            None,
            Some(head),
            &mut size,
        ) != NO_ERROR.0
        {
            return out;
        }
        let mut current = head;
        while !current.is_null() {
            let adapter = &*current;
            if adapter.OperStatus.0 == IfOperStatusUp.0
                && adapter.IfType != IF_TYPE_SOFTWARE_LOOPBACK
            {
                if let Some(ip) = first_ipv4(adapter) {
                    if let Ok(name) = adapter.FriendlyName.to_string() {
                        if !name.trim().is_empty() {
                            out.push((name, ip));
                        }
                    }
                }
            }
            current = adapter.Next;
        }
    }
    out
}

#[cfg(windows)]
fn first_ipv4(
    adapter: &windows::Win32::NetworkManagement::IpHelper::IP_ADAPTER_ADDRESSES_LH,
) -> Option<Ipv4Addr> {
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};

    unsafe {
        let mut unicast = adapter.FirstUnicastAddress;
        while !unicast.is_null() {
            let entry = &*unicast;
            let sockaddr = entry.Address.lpSockaddr;
            if !sockaddr.is_null() && (*sockaddr).sa_family == AF_INET {
                let inet = &*(sockaddr as *const SOCKADDR_IN);
                let b = inet.sin_addr.S_un.S_un_b;
                let ip = Ipv4Addr::new(b.s_b1, b.s_b2, b.s_b3, b.s_b4);
                if !ip.is_loopback() && !ip.is_link_local() {
                    return Some(ip);
                }
            }
            unicast = entry.Next;
        }
        None
    }
}

fn resolve() -> Vec<SocketAddr> {
    (HOST, PORT)
        .to_socket_addrs()
        .map(|it| it.collect())
        .unwrap_or_default()
}
