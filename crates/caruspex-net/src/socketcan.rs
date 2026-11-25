//! SocketCAN, the CAN layer of Linux: PEAK, Kvaser, CANable (candleLight),
//! Raspberry Pi CAN hats and every other adapter with a kernel driver.
//!
//! With CAN FD switched on (CAN_RAW_FD_FRAMES) every frame goes out as an
//! FD frame with bit rate switch, and classic and FD frames come in; the
//! interface must be set up for it (ip link set can0 up type can bitrate
//! 500000 dbitrate 2000000 fd on).
//!
//! The raw socket calls are unsafe; they stay in this module.
#![allow(unsafe_code)]

use std::mem::size_of;

use caruspex_i18n::l;

use crate::can_bus::{CanFrame, CanLink};

struct SocketCanLink {
    socket: libc::c_int,
    interface: String,
    fd: bool,
    /// The kernel said that the interface is gone.
    gone: bool,
}

/// Errors of a raw CAN socket whose interface was unplugged or taken down.
fn interface_gone(code: i32) -> bool {
    matches!(code, libc::ENODEV | libc::ENXIO | libc::ENETDOWN)
}

/// The smallest CAN FD data length that holds `n` bytes (DLC 9 to 15 are
/// 12, 16, 20, 24, 32, 48 and 64 bytes).
pub(crate) fn fd_length(n: usize) -> usize {
    match n {
        0..=8 => n,
        9..=12 => 12,
        13..=16 => 16,
        17..=20 => 20,
        21..=24 => 24,
        25..=32 => 32,
        33..=48 => 48,
        _ => 64,
    }
}

impl Drop for SocketCanLink {
    fn drop(&mut self) {
        // SAFETY: the socket belongs to this link and is closed once.
        unsafe { libc::close(self.socket) };
    }
}

impl SocketCanLink {
    /// After a failed call: remembers whether the interface is gone.
    fn note_error(&mut self) {
        let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        self.gone |= interface_gone(code);
    }

    fn send_fd(&mut self, frame: &CanFrame) -> bool {
        // SAFETY: canfd_frame is plain data, all zero is a valid value.
        let mut f: libc::canfd_frame = unsafe { std::mem::zeroed() };
        f.can_id = if frame.extended {
            frame.id | libc::CAN_EFF_FLAG
        } else {
            frame.id & libc::CAN_SFF_MASK
        };
        let size = frame.data.len().min(64);
        let length = fd_length(size);
        f.len = length as u8;
        f.flags = libc::CANFD_BRS as u8;
        f.data[..size].copy_from_slice(&frame.data[..size]);
        // SAFETY: writes exactly one canfd_frame from a valid buffer.
        let written = unsafe {
            libc::write(
                self.socket,
                (&f as *const libc::canfd_frame).cast::<libc::c_void>(),
                size_of::<libc::canfd_frame>(),
            )
        };
        if written < 0 {
            self.note_error();
        }
        written == size_of::<libc::canfd_frame>() as isize
    }
}

impl CanLink for SocketCanLink {
    fn fd(&self) -> bool {
        self.fd
    }

    fn send(&mut self, frame: &CanFrame) -> bool {
        if self.fd {
            return self.send_fd(frame);
        }
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
        if written < 0 {
            self.note_error();
        }
        written == size_of::<libc::can_frame>() as isize
    }

    fn lost(&mut self) -> bool {
        if !self.gone {
            let mut code: libc::c_int = 0;
            let mut length = size_of::<libc::c_int>() as libc::socklen_t;
            // SAFETY: code and length are valid for the size given.
            let got = unsafe {
                libc::getsockopt(
                    self.socket,
                    libc::SOL_SOCKET,
                    libc::SO_ERROR,
                    (&mut code as *mut libc::c_int).cast::<libc::c_void>(),
                    &mut length,
                )
            };
            self.gone = got == 0 && interface_gone(code);
        }
        self.gone
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
        // A classic frame and an FD frame begin alike (id, length); room for
        // the larger one, the size read tells which came.
        // SAFETY: canfd_frame is plain data, all zero is a valid value.
        let mut f: libc::canfd_frame = unsafe { std::mem::zeroed() };
        // SAFETY: reads at most one canfd_frame into a valid buffer.
        let got = unsafe {
            libc::read(
                self.socket,
                (&mut f as *mut libc::canfd_frame).cast::<libc::c_void>(),
                size_of::<libc::canfd_frame>(),
            )
        };
        let room = if got == size_of::<libc::canfd_frame>() as isize {
            64
        } else if got == size_of::<libc::can_frame>() as isize {
            8
        } else {
            if got < 0 {
                self.note_error();
            }
            return None;
        };
        if f.can_id & (libc::CAN_ERR_FLAG | libc::CAN_RTR_FLAG) != 0 {
            return None;
        }
        let extended = f.can_id & libc::CAN_EFF_FLAG != 0;
        let length = usize::from(f.len).min(room);
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
        if self.fd {
            format!("SocketCAN {} CAN FD", self.interface)
        } else {
            format!("SocketCAN {}", self.interface)
        }
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

pub(crate) fn open(interface: &str, fd: bool) -> Result<Box<dyn CanLink>, String> {
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
    if fd {
        let on: libc::c_int = 1;
        // SAFETY: CAN_RAW_FD_FRAMES takes one int.
        let set = unsafe {
            libc::setsockopt(
                s,
                libc::SOL_CAN_RAW,
                libc::CAN_RAW_FD_FRAMES,
                (&on as *const libc::c_int).cast::<libc::c_void>(),
                size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        if set < 0 {
            // SAFETY: the socket is ours.
            unsafe { libc::close(s) };
            return Err(format!(
                "{}{}",
                l(
                    "CAN FD geht an dieser Schnittstelle nicht: ",
                    "CAN FD does not work on this interface: "
                ),
                interface
            ));
        }
    }
    Ok(Box::new(SocketCanLink {
        socket: s,
        interface: interface.to_string(),
        fd,
        gone: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The interface for the tests over a real socket: the one named in
    /// CARUSPEX_TEST_VCAN (the Sanitizers job of the CI sets up vcan0 with
    /// room for CAN FD). Without it only the error paths are tested.
    fn test_interface() -> Option<String> {
        let name = std::env::var("CARUSPEX_TEST_VCAN")
            .ok()
            .filter(|n| !n.is_empty())?;
        interfaces().contains(&name).then_some(name)
    }

    #[test]
    fn socketcan_unknown_interfaces_are_refused() {
        assert!(open("nosuchcan9", false).is_err());
        assert!(open("nosuchcan9", true).is_err());
        // Longer than IFNAMSIZ: cut, never written past the name field.
        assert!(open(&"x".repeat(64), false).is_err());
        assert_eq!(fd_length(9), 12);
        assert_eq!(fd_length(64), 64);
        assert!(interface_gone(libc::ENODEV));
        assert!(!interface_gone(libc::EAGAIN));
        assert!(!interface_gone(libc::ENOBUFS));
    }

    // All traffic in one test, so no two tests read each other's frames.
    #[test]
    fn socketcan_frames_over_a_virtual_interface() {
        let Some(name) = test_interface() else {
            eprintln!("CARUSPEX_TEST_VCAN is not set up, skipping the traffic test");
            return;
        };
        let mut a = open(&name, false).unwrap();
        let mut b = open(&name, false).unwrap();
        assert_eq!(a.name(), format!("SocketCAN {name}"));
        for frame in [
            CanFrame {
                id: 0x7DF,
                extended: false,
                data: vec![0x02, 0x01, 0x0C],
            },
            CanFrame {
                id: 0x18DB_33F1,
                extended: true,
                data: vec![0x10, 0x14, 0x49, 0x02, 0x01, 0x57, 0x41, 0x55],
            },
            CanFrame {
                id: 0x7E8,
                extended: false,
                data: Vec::new(),
            },
        ] {
            assert!(a.send(&frame));
            assert_eq!(b.receive(1000), Some(frame));
        }
        // More than 8 bytes on a classic link: only the first 8 go out.
        let long = CanFrame {
            id: 0x7E0,
            extended: false,
            data: (0..12).collect(),
        };
        assert!(a.send(&long));
        assert_eq!(b.receive(1000).map(|f| f.data), Some((0..8).collect()));
        // The sender does not hear itself, and nothing else is waiting.
        assert_eq!(a.receive(50), None);
        assert_eq!(b.receive(50), None);

        // CAN FD: up to 64 bytes, padded to the next FD length.
        let mut fd_a = open(&name, true).unwrap();
        let mut fd_b = open(&name, true).unwrap();
        assert!(fd_a.fd());
        let frame = CanFrame {
            id: 0x7E0,
            extended: false,
            data: (1..=20).collect(),
        };
        assert!(fd_a.send(&frame), "vcan needs mtu 72 for CAN FD");
        assert_eq!(fd_b.receive(1000), Some(frame));
        let thirteen = CanFrame {
            id: 0x7E0,
            extended: false,
            data: vec![0xAA; 13],
        };
        assert!(fd_a.send(&thirteen));
        let got = fd_b.receive(1000).unwrap();
        assert_eq!(got.data.len(), 16);
        assert_eq!(got.data[..13], [0xAA; 13]);
        // Classic frames reach the FD sockets too; the classic socket saw
        // none of the FD frames, so the classic one is the next it reads.
        let classic = CanFrame {
            id: 0x7E8,
            extended: false,
            data: vec![1, 2, 3],
        };
        assert!(a.send(&classic));
        assert_eq!(fd_b.receive(1000), Some(classic.clone()));
        assert_eq!(b.receive(1000), Some(classic));
        assert_eq!(b.receive(50), None);
    }
}
