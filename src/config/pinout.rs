pub(crate) const MAX_PINS: usize = 13;
pub(crate) const MAX_CYCLES: usize = 21;
pub(crate) const UNUSED_CYCLE: u8 = 0xFF;

#[derive(Debug, Clone, Copy)]
pub enum Pin {
  RxTx0 = 0,
  RxTx1 = 1,
  RxTx2 = 2,
  RxTx3 = 3,
  RxTx4 = 4,
  RxTx5 = 5,
  RxTx6 = 6,
  RxTx7 = 7,
  Tx8 = 8,
  Tx9 = 9,
  Tx10 = 10,
  Tx11 = 11,
  Tx12 = 12,
}

impl Pin {
  fn is_a(self) -> bool {
    (self as u8) < 4
  }

  fn is_b(self) -> bool {
    (self as u8) >= 4 && (self as u8) < 8
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Pins {
  pins: [Pin; MAX_PINS],
  pub len: usize,
}

impl Pins {
  const fn new<const N: usize>(pins: [Pin; N]) -> Self {
    let mut full = [Pin::RxTx0; MAX_PINS];
    let mut i = 0;
    while i < N {
      full[i] = pins[i];
      i += 1;
    }
    Self { pins: full, len: N }
  }

  fn pins(&self) -> &[Pin] {
    &self.pins[..self.len]
  }

  const fn is_subset_of(&self, other: Pins) -> bool {
    let mut i = 0;
    while i < self.len {
      if !other.contains(self.pins[i]) {
        return false;
      }
      i += 1;
    }
    true
  }

  const fn contains(&self, pin: Pin) -> bool {
    let mut i = 0;
    while i < self.len {
      if self.pins[i] as u8 == pin as u8 {
        return true;
      }
      i += 1;
    }
    false
  }
}

/// Pin configuration for the trackpad sensor matrix (no padding).
#[derive(Debug, Clone, Copy)]
pub struct Pinout {
  pub rx: Pins,
  pub tx: Pins,
  pub alp_rx: Pins,
  pub alp_tx: Pins,
}

impl Pinout {
  pub const fn new<const RX: usize, const TX: usize, const ARX: usize, const ATX: usize>(
    rx: [Pin; RX],
    tx: [Pin; TX],
    alp_rx: [Pin; ARX],
    alp_tx: [Pin; ATX],
  ) -> Self {
    assert!(RX + TX <= MAX_PINS, "maximum 13 total Rx/Tx pins");
    let alp_rx = Pins::new(alp_rx);
    let alp_tx = Pins::new(alp_tx);
    let rx = Pins::new(rx);
    let tx = Pins::new(tx);
    assert!(alp_rx.is_subset_of(rx), "ALP rx should be a subset of main rx");
    assert!(alp_tx.is_subset_of(tx), "ALP tx should be a subset of main tx");
    Self { rx, tx, alp_rx, alp_tx }
  }

  pub const fn with_rxtx<const RX: usize, const TX: usize>(mut self, rx: [Pin; RX], tx: [Pin; TX]) -> Self {
    assert!(RX + TX <= MAX_PINS, "maximum 13 total Rx/Tx pins");
    self.rx = Pins::new(rx);
    self.tx = Pins::new(tx);
    assert!(self.alp_rx.is_subset_of(self.rx), "ALP rx should be a subset of main rx");
    assert!(self.alp_tx.is_subset_of(self.tx), "ALP tx should be a subset of main tx");
    self
  }

  pub const fn with_alp_rxtx<const RX: usize, const TX: usize>(mut self, rx: [Pin; RX], tx: [Pin; TX]) -> Self {
    self.alp_rx = Pins::new(rx);
    self.alp_tx = Pins::new(tx);
    assert!(self.alp_rx.is_subset_of(self.rx), "ALP rx should be a subset of main rx");
    assert!(self.alp_tx.is_subset_of(self.tx), "ALP tx should be a subset of main tx");
    self
  }

  pub(crate) fn mapping(&self) -> [u8; MAX_PINS + 1] {
    let mut out = [0; MAX_PINS + 1];
    for (idx, &pin) in self.rx.pins().iter().enumerate() {
      out[idx] = pin as u8;
    }
    for (offset, &pin) in self.tx.pins().iter().enumerate() {
      out[self.rx.len + offset] = pin as u8;
    }
    out
  }

  /// The trackpad channels need to be packed into cycles. The Azoteq PC GUI can be used to assist with this setup.
  /// Each cycle can simultaneously sense one channel from Prox block A (Rx0-3) and one from Prox block B (Rx4-7). They
  /// must be for the same Tx, and the channel numbers are packed into the cycle numbers (Cycle allocation registers)
  /// accordingly. A value of 255 for the channel number indicates no channel is allocated. It is best to select the
  /// Rxs as the even numbered sensors, so that optimal cycles/timeslot usage occurs. Similarly, a balanced number of
  /// sensors from A and B are optimal.
  ///
  /// The returned byte array is laid out exactly as expected by the cycle allocation registers (0x5D–0x7C): each cycle
  /// contributes three bytes `[0x05, prox_a, prox_b]`, and a trailing terminator byte (0x01) fills the final high byte
  /// of  the register window.
  pub(crate) fn cycles(&self) -> [u8; 64] {
    // Build cycles as (tx, prox_a_channel, prox_b_channel) tuples first
    let mut cycles = [(0u8, UNUSED_CYCLE, UNUSED_CYCLE); MAX_CYCLES];
    let mut cycle_count = 0usize;
    let mut channel_index = 0u8;

    for &tx in self.tx.pins() {
      for &rx in self.rx.pins() {
        if cycle_count >= MAX_CYCLES {
          break;
        }

        if !rx.is_a() && !rx.is_b() {
          channel_index += 1;
          continue;
        }

        // Find existing cycle for this TX with available slot, or create new one
        let cycle_idx = cycles[..cycle_count]
          .iter()
          .position(|&(cycle_tx, prox_a, prox_b)| {
            cycle_tx == tx as u8
              && if rx.is_a() {
                prox_a == UNUSED_CYCLE
              } else if rx.is_b() {
                prox_b == UNUSED_CYCLE
              } else {
                false
              }
          })
          .unwrap_or_else(|| {
            cycles[cycle_count] = (tx as u8, UNUSED_CYCLE, UNUSED_CYCLE);
            cycle_count += 1;
            cycle_count - 1
          });

        // Assign channel to appropriate slot (A or B)
        match rx.is_a() {
          true => cycles[cycle_idx].1 = channel_index,
          false => cycles[cycle_idx].2 = channel_index,
        }

        channel_index += 1;
      }
    }

    // Convert to register byte format: [0x05, prox_a, prox_b] per cycle
    let mut bytes = [0; 64];
    for i in 0..MAX_CYCLES {
      bytes[i * 3] = 0x05;
      bytes[i * 3 + 1] = cycles[i].1;
      bytes[i * 3 + 2] = cycles[i].2;
    }
    bytes[MAX_CYCLES * 3] = 0x01; // Final terminator, note not 0x05
    bytes
  }

  pub(crate) const fn alp_rx(&self) -> u8 {
    self.alp_rx.contains(Pin::RxTx0) as u8
      | (self.alp_rx.contains(Pin::RxTx1) as u8) << 1
      | (self.alp_rx.contains(Pin::RxTx2) as u8) << 2
      | (self.alp_rx.contains(Pin::RxTx3) as u8) << 3
      | (self.alp_rx.contains(Pin::RxTx4) as u8) << 4
      | (self.alp_rx.contains(Pin::RxTx5) as u8) << 5
      | (self.alp_rx.contains(Pin::RxTx6) as u8) << 6
      | (self.alp_rx.contains(Pin::RxTx7) as u8) << 7
  }

  pub(crate) const fn alp_tx(&self) -> u16 {
    self.alp_tx.contains(Pin::RxTx0) as u16
      | (self.alp_tx.contains(Pin::RxTx1) as u16) << 1
      | (self.alp_tx.contains(Pin::RxTx2) as u16) << 2
      | (self.alp_tx.contains(Pin::RxTx3) as u16) << 3
      | (self.alp_tx.contains(Pin::RxTx4) as u16) << 4
      | (self.alp_tx.contains(Pin::RxTx5) as u16) << 5
      | (self.alp_tx.contains(Pin::RxTx6) as u16) << 6
      | (self.alp_tx.contains(Pin::RxTx7) as u16) << 7
      | (self.alp_tx.contains(Pin::Tx8) as u16) << 8
      | (self.alp_tx.contains(Pin::Tx9) as u16) << 9
      | (self.alp_tx.contains(Pin::Tx10) as u16) << 10
      | (self.alp_tx.contains(Pin::Tx11) as u16) << 11
      | (self.alp_tx.contains(Pin::Tx12) as u16) << 12
  }
}

impl Default for Pinout {
  fn default() -> Self {
    Self::new([], [], [], [])
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn decoded_cycles(layout: &Pinout) -> [(u8, u8); MAX_CYCLES] {
    let bytes = layout.cycles();
    let mut out = [(UNUSED_CYCLE, UNUSED_CYCLE); MAX_CYCLES];
    let mut i = 0usize;
    while i < MAX_CYCLES {
      let base = i * 3;
      out[i] = (bytes[base + 1], bytes[base + 2]);
      i += 1;
    }
    out
  }

  // Helper that counts how many cycles actually carry at least one channel.
  fn count_non_empty(layout: &Pinout) -> usize {
    let mut n = 0usize;
    let cycles = decoded_cycles(layout);
    let mut i = 0;
    while i < cycles.len() {
      let (prox_a, prox_b) = cycles[i];
      if !(prox_a == UNUSED_CYCLE && prox_b == UNUSED_CYCLE) {
        n += 1;
      }
      i += 1;
    }
    n
  }

  #[test]
  fn pin_group_helpers_work() {
    assert!(Pin::RxTx0.is_a());
    assert!(Pin::RxTx3.is_a());
    assert!(!Pin::RxTx4.is_a());
    assert!(Pin::RxTx4.is_b());
    assert!(Pin::RxTx7.is_b());
    assert!(!Pin::Tx8.is_a());
    assert!(!Pin::Tx8.is_b());
  }

  #[test]
  fn pins_contains_and_subset() {
    let a = Pins::new([Pin::RxTx0, Pin::RxTx2, Pin::RxTx4]);
    let b = Pins::new([Pin::RxTx0, Pin::RxTx2, Pin::RxTx4, Pin::Tx8]);

    assert!(a.contains(Pin::RxTx0));
    assert!(!a.contains(Pin::RxTx1));

    assert!(a.is_subset_of(b)); // a ⊆ b
    assert!(!b.is_subset_of(a)); // b ⊄ a
  }

  #[test]
  fn mapping_places_rx_before_tx() {
    let layout = Pinout::new([Pin::RxTx0, Pin::RxTx2, Pin::RxTx4, Pin::RxTx6], [Pin::Tx8, Pin::Tx9, Pin::Tx10], [], []);

    let m = layout.mapping();

    assert_eq!(m[0..4], [0u8, 2u8, 4u8, 6u8]);
    assert_eq!(m[4..7], [8u8, 9u8, 10u8]);
  }

  #[test]
  fn alp_subset_ok() {
    let layout_ok =
      Pinout::new([Pin::RxTx0, Pin::RxTx2, Pin::RxTx4, Pin::RxTx6], [Pin::Tx8, Pin::Tx9, Pin::Tx10], [], [])
        .with_alp_rxtx([Pin::RxTx0, Pin::RxTx4], [Pin::Tx8]);

    // Touch it to avoid warnings and to ensure it built correctly.
    let m = layout_ok.mapping();
    assert_eq!(m[0], Pin::RxTx0 as u8);
  }

  #[test]
  #[should_panic] // ALP rx not subset of main rx
  fn alp_rx_not_subset_panics() {
    let _ = Pinout::new([Pin::RxTx0, Pin::RxTx2], [Pin::Tx8, Pin::Tx9], [], []).with_alp_rxtx([Pin::RxTx4], [Pin::Tx8]);
  }

  #[test]
  #[should_panic] // ALP tx not subset of main tx
  fn alp_tx_not_subset_panics() {
    let _ = Pinout::new([Pin::RxTx0, Pin::RxTx2], [Pin::Tx8], [], []).with_alp_rxtx([Pin::RxTx0], [Pin::Tx9]);
  }

  #[test]
  fn cycles_pair_a_and_b_same_tx_simple() {
    let layout = Pinout::new(
      [Pin::RxTx0, Pin::RxTx4], // A then B
      [Pin::Tx8, Pin::Tx9],
      [],
      [],
    );

    let cycles = decoded_cycles(&layout);

    // Only the first two cycles should be non-empty (one per TX)
    assert_eq!(count_non_empty(&layout), 2);

    // Expected channel sequence over nested (tx then rx):
    // tx8/rx0 -> ch0 (A) starts cycle 0
    // tx8/rx4 -> ch1 (B) backfills cycle 0
    // tx9/rx0 -> ch2 (A) starts cycle 1
    // tx9/rx4 -> ch3 (B) backfills cycle 1

    // Cycle 0
    assert_eq!(cycles[0].0, 0);
    assert_eq!(cycles[0].1, 1);

    // Cycle 1
    assert_eq!(cycles[1].0, 2);
    assert_eq!(cycles[1].1, 3);
  }

  #[test]
  fn cycles_bytes_are_register_ready() {
    let layout = Pinout::new([Pin::RxTx0, Pin::RxTx4], [Pin::Tx8, Pin::Tx9], [], []);

    let bytes = layout.cycles();

    // Every populated cycle shares the configured TX slot value.
    let separator = 0x05;
    assert_eq!(separator, bytes[3]);

    // Cycle 0 bytes (prox A channel 0, prox B channel 1)
    assert_eq!(bytes[1], 0);
    assert_eq!(bytes[2], 1);

    // Cycle 1 bytes (prox A channel 2, prox B channel 3)
    assert_eq!(bytes[4], 2);
    assert_eq!(bytes[5], 3);

    // Unused cycles retain the TX slot marker but use the UNUSED_CYCLE sentinel for both channels.
    assert_eq!(bytes[6], separator);
    assert_eq!(bytes[7], UNUSED_CYCLE);
    assert_eq!(bytes[8], UNUSED_CYCLE);

    // Final terminator byte is appended
    assert_eq!(bytes[MAX_CYCLES * 3], 0x01);
  }

  #[test]
  fn cycles_backfill_regardless_of_rx_order() {
    let layout = Pinout::new(
      [Pin::RxTx4, Pin::RxTx0], // B then A
      [Pin::Tx8],
      [],
      [],
    );

    let cycles = decoded_cycles(&layout);
    assert_eq!(count_non_empty(&layout), 1);

    // tx8/rx4 -> ch0 (B) makes cycle with A UNUSED_CYCLE
    // tx8/rx0 -> ch1 (A) backfills same cycle
    let c0 = cycles[0];
    assert_eq!(c0.0, 1);
    assert_eq!(c0.1, 0);
  }

  #[test]
  fn cycles_multiple_pairs_for_single_tx_are_packed() {
    let layout = Pinout::new(
      [Pin::RxTx0, Pin::RxTx4, Pin::RxTx2, Pin::RxTx6], // A0, B0, A1, B1
      [Pin::Tx8],
      [],
      [],
    );

    let cycles = decoded_cycles(&layout);
    assert_eq!(count_non_empty(&layout), 2);

    // cycle 0: A(ch0), B(ch1)
    assert_eq!(cycles[0].0, 0);
    assert_eq!(cycles[0].1, 1);

    // cycle 1: A(ch2), B(ch3)
    assert_eq!(cycles[1].0, 2);
    assert_eq!(cycles[1].1, 3);
  }

  #[test]
  fn cycles_with_only_a_side_create_distinct_cycles() {
    let layout = Pinout::new(
      [Pin::RxTx0, Pin::RxTx2, Pin::RxTx3], // all A-block
      [Pin::Tx8],
      [],
      [],
    );

    let cycles = decoded_cycles(&layout);
    assert_eq!(count_non_empty(&layout), 3);

    // Each A becomes its own cycle; B stays UNUSED_CYCLE
    assert_eq!(cycles[0].0, 0);
    assert_eq!(cycles[0].1, UNUSED_CYCLE);

    assert_eq!(cycles[1].0, 1);
    assert_eq!(cycles[1].1, UNUSED_CYCLE);

    assert_eq!(cycles[2].0, 2);
    assert_eq!(cycles[2].1, UNUSED_CYCLE);
  }

  #[test]
  fn cycles_span_multiple_txs_and_preserve_channel_indexing() {
    // A0, B0, A1 across two TXs.
    let layout = Pinout::new([Pin::RxTx0, Pin::RxTx4, Pin::RxTx2], [Pin::Tx8, Pin::Tx9], [], []);

    let cycles = decoded_cycles(&layout);

    // We expect 4 non-empty cycles overall:
    // TX8: (A0,B0) -> cycle 0, then (A1,UNUSED_CYCLE) -> cycle 1
    // TX9: (A0,B0) -> cycle 2, then (A1,UNUSED_CYCLE) -> cycle 3
    assert!(count_non_empty(&layout) >= 4);

    // Check the first three cycles explicitly (pairing & indexing are the key behaviors)
    assert_eq!(cycles[0].0, 0);
    assert_eq!(cycles[0].1, 1);

    assert_eq!(cycles[1].0, 2);
    assert_eq!(cycles[1].1, UNUSED_CYCLE);

    assert_eq!(cycles[2].0, 3);
    assert_eq!(cycles[2].1, 4);
  }
}
