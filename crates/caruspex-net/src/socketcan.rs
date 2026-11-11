//! SocketCAN, the CAN layer of Linux: PEAK, Kvaser, CANable (candleLight),
//! Raspberry Pi CAN hats and every other adapter with a kernel driver.
//!
//! The raw socket calls are unsafe; they stay in this module.
#![allow(unsafe_code)]

use std::mem::size_of;

use caruspex_i18n::l;

use crate::can_bus::{CanFrame, CanLink};

struct SocketCanLink {
    socket: libc::c_int,
    interface: String,
}

impl Drop for SocketCanLink {
    fn drop(&mut self) {
        // SAFETY: the socket belongs to this link and is closed once.
        unsafe { libc::close(self.socket) };
    }
}

impl CanLink for SocketCanLink {
    fn send(&mut self, frame: &CanFrame) -> bool {
        // SAFETY: can_frame is plain data, all zero is a valid value.
        let mut f: libc::can_frame = unsafe { std::mem::zeroed() };
        f.can_id = if frame.extended {
            frame.id | libc::CAN_EFF_FLAG
        } else {
            frame.id & libc::CAN_SFF_MASK
        };
        let size = frame.data.len().min(8);
        f.can_dlc = size as u8;
        f.data[..size].copy_from_slice(&frame.data[..size]);
        // SAFETY: writes exactly one can_frame from a valid buffer.
        let written = unsafe {
            libc::write(
                self.socket,
                (&f as *const libc::can_frame).cast::<libc::c_void>(),
                size_of::<libc::can_frame>(),
            )
        };
        written == size_of::<libc::can_frame>() as isize
    }

    fn receive(&mut self, timeout_ms: u64) -> Option<CanFrame> {
        let mut p = libc::pollfd {
            fd: self.socket,
            events: libc::POLLIN,
            revents: 0,
        };
        let timeout = libc::c_int::try_from(timeout_ms).unwrap_or(libc::c_int::MAX);
        // SAFETY: one valid pollfd.
        if unsafe { libc::poll(&mut p, 1, timeout) } <= 0 {
            return None;
        }
        // SAFETY: can_frame is plain data, all zero is a valid value.
        let mut f: libc::can_frame = unsafe { std::mem::zeroed() };
        // SAFETY: reads at most one can_frame into a valid buffer.
        let got = unsafe {
            libc::read(
                self.socket,
                (&mut f as *mut libc::can_frame).cast::<libc::c_void>(),
                size_of::<libc::can_frame>(),
            )
        };
        if got != size_of::<libc::can_frame>() as isize {
            return None;
        }
        if f.can_id & (libc::CAN_ERR_FLAG | libc::CAN_RTR_FLAG) != 0 {
            return None;
        }
        let extended = f.can_id & libc::CAN_EFF_FLAG != 0;
        let length = usize::from(f.can_dlc).min(8);
        Some(CanFrame {
            extended,
            id: if extended {
                f.can_id & libc::CAN_EFF_MASK
            } else {
                f.can_id & libc::CAN_SFF_MASK
            },
            data: f.data[..length].to_vec(),
        })
    }

    fn name(&self) -> String {
        format!("SocketCAN {}", self.interface)
    }
}

pub(crate) fn interfaces() -> Vec<String> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir("/sys/class/net") else {
        return out;
    };
    for entry in dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(kind) = std::fs::read_to_string(format!("/sys/class/net/{name}/type")) else {
            continue;
        };
        if kind.trim().parse::<i32>() == Ok(280) {
            out.push(name); // ARPHRD_CAN
        }
    }
    out
}

pub(crate) fn open(interface: &str) -> Result<Box<dyn CanLink>, String> {
    // SAFETY: plain socket call.
    let s = unsafe { libc::socket(libc::PF_CAN, libc::SOCK_RAW, libc::CAN_RAW) };
    if s < 0 {
        return Err(l(
            "SocketCAN ist nicht verfügbar.",
            "SocketCAN is not available.",
        )
        .into());
    }
    // SAFETY: ifreq is plain data, all zero is a valid value.
    let mut ifr: libc::ifreq = unsafe { std::mem::zeroed() };
    for (slot, byte) in ifr
        .ifr_name
        .iter_mut()
        .zip(interface.bytes().take(libc::IFNAMSIZ - 1))
    {
        *slot = byte as libc::c_char;
    }
    // SAFETY: SIOCGIFINDEX reads the name and writes the index into ifr.
    if unsafe { libc::ioctl(s, libc::SIOCGIFINDEX, &mut ifr) } < 0 {
        // SAFETY: the socket is ours.
        unsafe { libc::close(s) };
        return Err(format!(
            "{}{}{}",
            l(
                "Die CAN-Schnittstelle gibt es nicht: ",
                "No such CAN interface: "
            ),
            interface,
            l(
                ". Erst einrichten: ip link set can0 up type can bitrate 500000",
                ". Set it up first: ip link set can0 up type can bitrate 500000"
            )
        ));
    }
    // SAFETY: sockaddr_can is plain data, all zero is a valid value.
    let mut address: libc::sockaddr_can = unsafe { std::mem::zeroed() };
    address.can_family = libc::AF_CAN as libc::sa_family_t;
    // SAFETY: the union member written by SIOCGIFINDEX.
    address.can_ifindex = unsafe { ifr.ifr_ifru.ifru_ifindex };
    // SAFETY: binds to a valid address of the right size.
    let bound = unsafe {
        libc::bind(
            s,
            (&address as *const libc::sockaddr_can).cast::<libc::sockaddr>(),
            size_of::<libc::sockaddr_can>() as libc::socklen_t,
        )
    };
    if bound < 0 {
        // SAFETY: the socket is ours.
        unsafe { libc::close(s) };
        return Err(format!(
            "{}{}",
            l(
                "Die CAN-Schnittstelle lässt sich nicht öffnen: ",
                "The CAN interface cannot be opened: "
            ),
            interface
        ));
    }
    Ok(Box::new(SocketCanLink {
        socket: s,
        interface: interface.to_string(),
    }))
}
