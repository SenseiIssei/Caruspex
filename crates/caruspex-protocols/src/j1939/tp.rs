//! The J1939 transport protocol: messages of 9 to 1785 bytes in up to 255
//! packets of seven bytes.
//!
//! Two ways: a broadcast announcement (BAM) followed by the data packets
//! to everybody, without any answer; or, to one receiver, a request to
//! send (RTS), which the receiver answers with clear to send (CTS) for some
//! packets, and at the end with an acknowledgement (end of message). The
//! connection management frames (TP.CM, PGN 60416) carry the size, the
//! packet count and the PGN of the message; the data frames (TP.DT, PGN
//! 60160) a sequence number from 1 and seven bytes each.

use std::collections::BTreeMap;

use super::{GLOBAL, J1939Id, PGN_TP_CM, PGN_TP_DT};

pub(crate) const CM_RTS: u8 = 16;
pub(crate) const CM_CTS: u8 = 17;
pub(crate) const CM_END_OF_MESSAGE: u8 = 19;
pub(crate) const CM_BAM: u8 = 32;
pub(crate) const CM_ABORT: u8 = 255;

/// One connection management frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpCm {
    Rts {
        size: u16,
        packets: u8,
        /// Most packets the sender takes per CTS; 255 for no limit.
        max_per_cts: u8,
        pgn: u32,
    },
    Cts {
        packets: u8,
        next: u8,
        pgn: u32,
    },
    EndOfMessage {
        size: u16,
        packets: u8,
        pgn: u32,
    },
    Bam {
        size: u16,
        packets: u8,
        pgn: u32,
    },
    Abort {
        reason: u8,
        pgn: u32,
    },
}

fn pgn_of(data: &[u8]) -> u32 {
    u32::from(data[5]) | (u32::from(data[6]) << 8) | (u32::from(data[7]) << 16)
}

impl TpCm {
    pub fn parse(data: &[u8]) -> Option<TpCm> {
        if data.len() < 8 {
            return None;
        }
        let size = u16::from_le_bytes([data[1], data[2]]);
        let pgn = pgn_of(data);
        Some(match data[0] {
            CM_RTS => TpCm::Rts {
                size,
                packets: data[3],
                max_per_cts: data[4],
                pgn,
            },
            CM_CTS => TpCm::Cts {
                packets: data[1],
                next: data[2],
                pgn,
            },
            CM_END_OF_MESSAGE => TpCm::EndOfMessage {
                size,
                packets: data[3],
                pgn,
            },
            CM_BAM => TpCm::Bam {
                size,
                packets: data[3],
                pgn,
            },
            CM_ABORT => TpCm::Abort {
                reason: data[1],
                pgn,
            },
            _ => return None,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let pgn_bytes = |pgn: u32| [pgn as u8, (pgn >> 8) as u8, (pgn >> 16) as u8];
        let (head, pgn) = match *self {
            TpCm::Rts {
                size,
                packets,
                max_per_cts,
                pgn,
            } => {
                let s = size.to_le_bytes();
                ([CM_RTS, s[0], s[1], packets, max_per_cts], pgn)
            }
            TpCm::Cts { packets, next, pgn } => ([CM_CTS, packets, next, 0xFF, 0xFF], pgn),
            TpCm::EndOfMessage { size, packets, pgn } => {
                let s = size.to_le_bytes();
                ([CM_END_OF_MESSAGE, s[0], s[1], packets, 0xFF], pgn)
            }
            TpCm::Bam { size, packets, pgn } => {
                let s = size.to_le_bytes();
                ([CM_BAM, s[0], s[1], packets, 0xFF], pgn)
            }
            TpCm::Abort { reason, pgn } => ([CM_ABORT, reason, 0xFF, 0xFF, 0xFF], pgn),
        };
        let mut out = head.to_vec();
        out.extend_from_slice(&pgn_bytes(pgn));
        out
    }
}

/// The data packets of a message: sequence number from 1, seven bytes,
/// the last one filled up with FF.
#[cfg(test)]
pub(crate) fn data_packets(payload: &[u8]) -> Vec<Vec<u8>> {
    payload
        .chunks(7)
        .enumerate()
        .map(|(i, chunk)| {
            let mut frame = vec![(i + 1) as u8];
            frame.extend_from_slice(chunk);
            frame.resize(8, 0xFF);
            frame
        })
        .collect()
}

/// Number of packets for a message of `size` bytes.
#[cfg(test)]
pub(crate) fn packet_count(size: usize) -> u8 {
    size.div_ceil(7).min(255) as u8
}

struct Transfer {
    pgn: u32,
    size: usize,
    packets: Vec<Option<[u8; 7]>>,
}

impl Transfer {
    fn new(pgn: u32, size: u16, packets: u8) -> Transfer {
        Transfer {
            pgn,
            size: usize::from(size),
            packets: vec![None; usize::from(packets)],
        }
    }
    fn complete(&self) -> bool {
        self.packets.iter().all(Option::is_some)
    }
    /// The first packet still missing, counted from 1.
    fn first_missing(&self) -> Option<u8> {
        self.packets
            .iter()
            .position(Option::is_none)
            .map(|i| (i + 1) as u8)
    }
    fn data(&self) -> Vec<u8> {
        let mut out: Vec<u8> = self.packets.iter().flatten().flatten().copied().collect();
        out.truncate(self.size);
        out
    }
}

/// What a frame meant for the collector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TpEvent {
    /// A whole message arrived.
    Complete {
        source: u8,
        destination: u8,
        pgn: u32,
        data: Vec<u8>,
    },
    /// Somebody wants to send a message to us: answer with CTS.
    RequestToSend {
        source: u8,
        size: u16,
        packets: u8,
        max_per_cts: u8,
        pgn: u32,
    },
    /// The sender gave up.
    Aborted { source: u8, pgn: u32 },
}

/// Puts multi-packet messages back together from the frames of the bus:
/// every BAM, and the RTS/CTS transfers to `us`.
#[derive(Default)]
pub struct TpCollector {
    transfers: BTreeMap<(u8, u8), Transfer>,
}

impl TpCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes one TP.CM or TP.DT frame; other frames are ignored.
    pub fn feed(&mut self, id: J1939Id, data: &[u8], us: Option<u8>) -> Option<TpEvent> {
        let for_us = us == Some(id.destination);
        if id.destination != GLOBAL && !for_us {
            return None;
        }
        let key = (id.source, id.destination);
        match id.pgn {
            PGN_TP_CM => match TpCm::parse(data)? {
                TpCm::Bam { size, packets, pgn } if id.destination == GLOBAL => {
                    if packets == 0 {
                        return None;
                    }
                    self.transfers
                        .insert(key, Transfer::new(pgn, size, packets));
                    None
                }
                TpCm::Rts {
                    size,
                    packets,
                    max_per_cts,
                    pgn,
                } if for_us => {
                    if packets == 0 {
                        return None;
                    }
                    self.transfers
                        .insert(key, Transfer::new(pgn, size, packets));
                    Some(TpEvent::RequestToSend {
                        source: id.source,
                        size,
                        packets,
                        max_per_cts,
                        pgn,
                    })
                }
                TpCm::Abort { pgn, .. } => {
                    self.transfers.remove(&key);
                    Some(TpEvent::Aborted {
                        source: id.source,
                        pgn,
                    })
                }
                _ => None,
            },
            PGN_TP_DT => {
                let transfer = self.transfers.get_mut(&key)?;
                let sequence = usize::from(*data.first()?);
                if sequence == 0 || sequence > transfer.packets.len() || data.len() < 2 {
                    return None;
                }
                let mut chunk = [0xFF; 7];
                let bytes = &data[1..data.len().min(8)];
                chunk[..bytes.len()].copy_from_slice(bytes);
                transfer.packets[sequence - 1] = Some(chunk);
                if !transfer.complete() {
                    return None;
                }
                let transfer = self.transfers.remove(&key)?;
                Some(TpEvent::Complete {
                    source: id.source,
                    destination: id.destination,
                    pgn: transfer.pgn,
                    data: transfer.data(),
                })
            }
            _ => None,
        }
    }

    /// The first packet still missing of the transfer from `source` to
    /// `destination`, counted from 1; `None` when there is no such transfer.
    pub fn missing(&self, source: u8, destination: u8) -> Option<u8> {
        self.transfers.get(&(source, destination))?.first_missing()
    }

    /// Forgets a transfer (after an abort sent by us).
    pub(crate) fn drop_transfer(&mut self, source: u8, destination: u8) {
        self.transfers.remove(&(source, destination));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cm_frames_round_trip() {
        for cm in [
            TpCm::Rts {
                size: 18,
                packets: 3,
                max_per_cts: 0xFF,
                pgn: 0xFEEC,
            },
            TpCm::Cts {
                packets: 3,
                next: 1,
                pgn: 0xFEEC,
            },
            TpCm::EndOfMessage {
                size: 18,
                packets: 3,
                pgn: 0xFEEC,
            },
            TpCm::Bam {
                size: 10,
                packets: 2,
                pgn: 0xFECA,
            },
            TpCm::Abort {
                reason: 3,
                pgn: 0xFEEC,
            },
        ] {
            let bytes = cm.encode();
            assert_eq!(bytes.len(), 8);
            assert_eq!(TpCm::parse(&bytes), Some(cm));
        }
        assert_eq!(
            TpCm::Bam {
                size: 10,
                packets: 2,
                pgn: 0xFECA
            }
            .encode(),
            vec![0x20, 0x0A, 0x00, 0x02, 0xFF, 0xCA, 0xFE, 0x00]
        );
    }

    #[test]
    fn bam_is_put_together_even_out_of_order() {
        let payload: Vec<u8> = (1..=10).collect();
        let mut collector = TpCollector::new();
        let cm = J1939Id::parse(0x1CEC_FF00);
        let dt = J1939Id::parse(0x1CEB_FF00);
        let bam = TpCm::Bam {
            size: 10,
            packets: packet_count(10),
            pgn: 0xFECA,
        }
        .encode();
        assert_eq!(collector.feed(cm, &bam, None), None);
        let packets = data_packets(&payload);
        assert_eq!(packets.len(), 2);
        assert_eq!(collector.feed(dt, &packets[1], None), None);
        assert_eq!(collector.missing(0x00, GLOBAL), Some(1));
        assert_eq!(
            collector.feed(dt, &packets[0], None),
            Some(TpEvent::Complete {
                source: 0x00,
                destination: GLOBAL,
                pgn: 0xFECA,
                data: payload
            })
        );
    }

    #[test]
    fn rts_only_when_meant_for_us() {
        let mut collector = TpCollector::new();
        let rts = TpCm::Rts {
            size: 18,
            packets: 3,
            max_per_cts: 0xFF,
            pgn: 0xFEEC,
        }
        .encode();
        // To somebody else.
        assert_eq!(
            collector.feed(J1939Id::parse(0x1CEC_2100), &rts, Some(0xF9)),
            None
        );
        let event = collector.feed(J1939Id::parse(0x1CEC_F900), &rts, Some(0xF9));
        assert!(matches!(
            event,
            Some(TpEvent::RequestToSend { packets: 3, .. })
        ));
        let payload = b"1M2AX09C5JM000001*".to_vec();
        let dt = J1939Id::parse(0x1CEB_F900);
        let mut done = None;
        for p in data_packets(&payload) {
            done = collector.feed(dt, &p, Some(0xF9));
        }
        assert!(matches!(done, Some(TpEvent::Complete { data, .. }) if data == payload));
    }
}
