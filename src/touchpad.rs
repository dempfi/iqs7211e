//! High-level touch event facade transforming raw IQS7211E snapshots into
//! ergonomic contact updates. Available when the `touchpad` Cargo feature is
//! enabled.
//!
//! This module provides a streamlined touchpad API following established patterns
//! from multi-touch interfaces. The API is designed to be:
//!
//! - **Event-driven**: React to touch changes rather than polling state
//! - **Multi-touch aware**: Track up to 2 simultaneous contacts
//! - **Gesture integrated**: Seamlessly handle both raw touches and gestures
//! - **Memory efficient**: Zero-allocation operation suitable for embedded systems
//!
//! # Usage Patterns
//!
//! ## Basic Event Handling
//!
//! ```no_run
//! # use embedded_hal_async::{digital::Wait, i2c::{I2c, SevenBitAddress}};
//! # use iqs7211e::{Config, Touchpad, TouchPhase};
//! # async fn example<I2C, RDY, E>(controller: iqs7211e::Iqs7211e<I2C, RDY>) -> Result<(), iqs7211e::Error<E>>
//! # where I2C: I2c<SevenBitAddress, Error = E>, RDY: Wait
//! # {
//! let mut touchpad = Touchpad::new(controller);
//!
//! loop {
//!     let report = touchpad.next_frame().await?;
//!
//!     // Handle gestures
//!     if let Some(gesture) = report.gesture() {
//!         match gesture {
//!             iqs7211e::Gesture::SingleTap => println!("Tap detected"),
//!             iqs7211e::Gesture::SwipeXPositive => println!("Swipe right"),
//!             _ => println!("Other gesture: {:?}", gesture),
//!         }
//!     }
//!
//!     // Handle touch events
//!     for contact in report.contacts().iter() {
//!         match contact.phase {
//!             TouchPhase::Start => println!("Touch started at ({}, {})", contact.point.x, contact.point.y),
//!             TouchPhase::Move => println!("Touch moved to ({}, {})", contact.point.x, contact.point.y),
//!             TouchPhase::End => println!("Touch ended"),
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Advanced Touch Analysis
//!
//! ```no_run
//! # use embedded_hal_async::{digital::Wait, i2c::{I2c, SevenBitAddress}};
//! # use iqs7211e::{Config, Touchpad, TouchPhase, SwipeDirection};
//! # use iqs7211e::touchpad::utils;
//! # async fn advanced_example<I2C, RDY, E>(controller: iqs7211e::Iqs7211e<I2C, RDY>) -> Result<(), iqs7211e::Error<E>>
//! # where I2C: I2c<SevenBitAddress, Error = E>, RDY: Wait
//! # {
//! let mut touchpad = Touchpad::new(controller);
//! let mut last_primary_point = None;
//!
//! loop {
//!     let report = touchpad.next_frame().await?;
//!
//!     // Detect session transitions
//!     if report.is_session_start() {
//!         println!("New touch session started");
//!     } else if report.is_session_end() {
//!         println!("Touch session ended");
//!         last_primary_point = None;
//!     }
//!
//!     // Analyze multi-touch interactions
//!     if report.is_multi_touch() {
//!         if let Some(centroid) = report.snapshot().centroid() {
//!             println!("Multi-touch centroid at ({}, {})", centroid.x, centroid.y);
//!         }
//!     }
//!
//!     // Track movement with utilities
//!     if let Some(primary_contact) = report.contacts().primary() {
//!         if primary_contact.phase.is_move() {
//!             if let Some(last_point) = last_primary_point {
//!                 if utils::is_significant_movement(last_point, primary_contact.point, 10) {
//!                     let direction = utils::movement_direction(last_point, primary_contact.point);
//!                     println!("Significant movement: {}", direction.as_str());
//!                 }
//!             }
//!         }
//!         last_primary_point = Some(primary_contact.point);
//!     }
//!
//!     // Enhanced gesture classification
//!     if report.is_swipe_gesture() {
//!         if let Some(direction) = report.swipe_direction() {
//!             println!("Swipe gesture: {} ({})",
//!                 if direction.is_horizontal() { "horizontal" } else { "vertical" },
//!                 direction.as_str()
//!             );
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Stream-based Processing
//!
//! ```no_run
//! # use embedded_hal_async::{digital::Wait, i2c::{I2c, SevenBitAddress}};
//! # use iqs7211e::{Config, Touchpad};
//! # async fn stream_example<I2C, RDY, E>(mut touchpad: Touchpad<I2C, RDY>) -> Result<(), iqs7211e::Error<E>>
//! # where I2C: I2c<SevenBitAddress, Error = E>, RDY: Wait
//! # {
//! let mut stream = touchpad.stream();
//!
//! // Process events in a stream-like manner
//! while let Some(report) = stream.next().await? {
//!     if report.has_activity() {
//!         println!("Activity detected: {} touches, gesture: {:?}",
//!             report.snapshot().count(),
//!             report.gesture()
//!         );
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{defs::*, event::*, Error, Iqs7211e};

/// Indicates how a finger changed compared to the previous report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum TouchPhase {
  /// A new finger contact appeared on the surface.
  Start,
  /// An existing finger moved or changed pressure.
  Move,
  /// A finger was lifted off the surface.
  End,
}

impl TouchPhase {
  /// Returns `true` if this represents a new touch.
  pub const fn is_start(self) -> bool {
    matches!(self, TouchPhase::Start)
  }

  /// Returns `true` if this represents a touch movement.
  pub const fn is_move(self) -> bool {
    matches!(self, TouchPhase::Move)
  }

  /// Returns `true` if this represents a touch ending.
  pub const fn is_end(self) -> bool {
    matches!(self, TouchPhase::End)
  }

  /// Returns a human-readable string representation of the phase.
  pub const fn as_str(self) -> &'static str {
    match self {
      TouchPhase::Start => "start",
      TouchPhase::Move => "move",
      TouchPhase::End => "end",
    }
  }
}

/// Identifies which hardware-maintained contact slot produced an update.
///
/// The IQS7211E simultaneously tracks up to two active contacts. The firmware
/// assigns them to the primary or secondary slot, and can reshuffle that mapping
/// as fingers appear or vanish. Downstream code should therefore treat the slots
/// as unnamed touch channels rather than a specific finger index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ContactSlot {
  /// Slot used for the first active contact reported by the controller.
  Primary,
  /// Slot used for the optional second contact reported by the controller.
  Secondary,
}

impl ContactSlot {
  /// Returns `true` if this is the primary contact slot.
  pub const fn is_primary(self) -> bool {
    matches!(self, ContactSlot::Primary)
  }

  /// Returns `true` if this is the secondary contact slot.
  pub const fn is_secondary(self) -> bool {
    matches!(self, ContactSlot::Secondary)
  }

  /// Returns a human-readable string representation of the slot.
  pub const fn as_str(self) -> &'static str {
    match self {
      ContactSlot::Primary => "primary",
      ContactSlot::Secondary => "secondary",
    }
  }
}

/// Position and pressure details for a single contact point.
///
/// This is a type alias for [`Finger`] from the event module, avoiding duplication
/// while providing a more domain-appropriate name in the touchpad context.
///
/// # Coordinate System
/// - `x` and `y` coordinates follow the device's configured coordinate system
/// - Values of `0xFFFF` for coordinates indicate an invalid/absent touch
/// - `strength` and `area` are device-specific units representing touch pressure and contact size
pub use crate::event::Finger;

/// Description of a single contact transition in the latest report.
///
/// Each time [`Touchpad::next_frame`] completes, it compares the new contact
/// snapshots to the previous ones. For every slot where the state changed a
/// `Touch` is emitted that captures the slot, phase, and the most recent
/// coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct Touch {
  pub slot: ContactSlot,
  pub phase: TouchPhase,
  pub point: Finger,
}

impl Touch {
  pub const fn new(slot: ContactSlot, phase: TouchPhase, point: Finger) -> Self {
    Self { slot, phase, point }
  }

  pub fn is_primary(&self) -> bool {
    matches!(self.slot, ContactSlot::Primary)
  }

  pub fn is_secondary(&self) -> bool {
    matches!(self.slot, ContactSlot::Secondary)
  }
}

/// Current state of all active contacts as seen in the latest report.
///
/// Convenience accessors (`primary`, `secondary`, [`State::iter`]) make
/// it easy to inspect the current contact locations without handling arrays or
/// low-level structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct State {
  primary: Option<Finger>,
  secondary: Option<Finger>,
}

impl State {
  pub const fn new(primary: Option<Finger>, secondary: Option<Finger>) -> Self {
    Self { primary, secondary }
  }

  /// Get the primary contact point, if present.
  pub fn primary(&self) -> Option<Finger> {
    self.primary
  }

  /// Get the secondary contact point, if present.
  pub fn secondary(&self) -> Option<Finger> {
    self.secondary
  }

  /// Get a contact point by slot, if present.
  pub fn get(&self, slot: ContactSlot) -> Option<Finger> {
    match slot {
      ContactSlot::Primary => self.primary,
      ContactSlot::Secondary => self.secondary,
    }
  }

  /// Iterate over all active contact points.
  pub fn iter(&self) -> impl Iterator<Item = Finger> + '_ {
    self.primary.into_iter().chain(self.secondary)
  }

  /// Count the number of active contacts.
  pub fn count(&self) -> usize {
    self.iter().count()
  }

  /// Check if there are any active contacts.
  pub fn is_empty(&self) -> bool {
    self.primary.is_none() && self.secondary.is_none()
  }

  /// Check if there are multiple active contacts.
  pub fn is_multi_touch(&self) -> bool {
    self.primary.is_some() && self.secondary.is_some()
  }

  /// Get the centroid (average position) of all active contacts.
  /// Returns `None` if no contacts are active.
  pub fn centroid(&self) -> Option<Finger> {
    match (self.primary, self.secondary) {
      (None, None) => None,
      (Some(point), None) | (None, Some(point)) => Some(point),
      (Some(p1), Some(p2)) => {
        // Average the two points
        Some(Finger::new(
          ((p1.x as u32 + p2.x as u32) / 2) as u16,
          ((p1.y as u32 + p2.y as u32) / 2) as u16,
          ((p1.strength as u32 + p2.strength as u32) / 2) as u16,
          ((p1.area as u32 + p2.area as u32) / 2) as u16,
        ))
      }
    }
  }
}

/// Collection of contact transitions reported by the device.
///
/// At most one transition exists for each slot. Applications can cherry-pick
/// the primary or secondary contact, or iterate over both via
/// [`Changes::iter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct Changes {
  primary: Option<Touch>,
  secondary: Option<Touch>,
}

impl Changes {
  pub const fn new(primary: Option<Touch>, secondary: Option<Touch>) -> Self {
    Self { primary, secondary }
  }

  /// Get the primary contact event, if any change occurred.
  pub fn primary(&self) -> Option<Touch> {
    self.primary
  }

  /// Get the secondary contact event, if any change occurred.
  pub fn secondary(&self) -> Option<Touch> {
    self.secondary
  }

  /// Get a contact event by slot, if any change occurred.
  pub fn get(&self, slot: ContactSlot) -> Option<Touch> {
    match slot {
      ContactSlot::Primary => self.primary,
      ContactSlot::Secondary => self.secondary,
    }
  }

  /// Iterate over all contact changes in this report.
  pub fn iter(&self) -> impl Iterator<Item = Touch> + '_ {
    self.primary.into_iter().chain(self.secondary)
  }

  /// Check if no contact changes occurred.
  pub fn is_empty(&self) -> bool {
    self.primary.is_none() && self.secondary.is_none()
  }

  /// Count the number of contact changes.
  pub fn count(&self) -> usize {
    let mut count = 0;
    if self.primary.is_some() {
      count += 1;
    }
    if self.secondary.is_some() {
      count += 1;
    }
    count
  }

  /// Check if there are any contact starts (new touches).
  pub fn has_starts(&self) -> bool {
    self.iter().any(|contact| matches!(contact.phase, TouchPhase::Start))
  }

  /// Check if there are any contact ends (lifted touches).
  pub fn has_ends(&self) -> bool {
    self.iter().any(|contact| matches!(contact.phase, TouchPhase::End))
  }

  /// Check if there are any contact moves (moved touches).
  pub fn has_moves(&self) -> bool {
    self.iter().any(|contact| matches!(contact.phase, TouchPhase::Move))
  }

  /// Get all contacts matching a specific phase.
  pub fn contacts_with_phase(&self, phase: TouchPhase) -> impl Iterator<Item = Touch> + '_ {
    self.iter().filter(move |contact| contact.phase == phase)
  }
}

/// High-level summary describing changes observed on the touchpad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct Frame {
  pub info: InfoFlags,
  pub gesture: Option<Gesture>,
  pub events: Changes,
  pub state: State,
}

impl Frame {
  pub const fn new(info: InfoFlags, gesture: Option<Gesture>, events: Changes, state: State) -> Self {
    Self { info, gesture, events, state }
  }

  /// Return the raw [`InfoFlags`] block captured with this frame.
  pub const fn info(&self) -> InfoFlags {
    self.info
  }

  /// Return the gesture reported with this frame, if any.
  pub const fn gesture(&self) -> Option<Gesture> {
    self.gesture
  }

  /// Return the set of contact transitions contained in this frame.
  pub const fn contacts(&self) -> Changes {
    self.events
  }

  /// Return the consolidated snapshot of active touches.
  pub const fn snapshot(&self) -> State {
    self.state
  }

  /// Check if this report contains any touch activity (events or gestures).
  pub fn has_activity(&self) -> bool {
    self.gesture.is_some() || !self.events.is_empty()
  }

  /// Check if this report indicates a new touch session starting.
  /// A new session starts when the first touch contact begins.
  pub fn is_session_start(&self) -> bool {
    self.events.has_starts() && self.state.count() == 1
  }

  /// Check if this report indicates a touch session ending.
  /// A session ends when the last touch contact ends.
  pub fn is_session_end(&self) -> bool {
    self.events.has_ends() && self.state.is_empty()
  }

  /// Check if this report contains multi-touch activity.
  pub fn is_multi_touch(&self) -> bool {
    self.state.is_multi_touch()
  }

  /// Check if this report contains a tap gesture (single, double, or triple).
  pub fn is_tap_gesture(&self) -> bool {
    match self.gesture {
      Some(Gesture::SingleTap) | Some(Gesture::DoubleTap) | Some(Gesture::TripleTap) => true,
      _ => false,
    }
  }

  /// Check if this report contains a swipe gesture (any direction).
  pub fn is_swipe_gesture(&self) -> bool {
    match self.gesture {
      Some(Gesture::SwipeXPositive)
      | Some(Gesture::SwipeXNegative)
      | Some(Gesture::SwipeYPositive)
      | Some(Gesture::SwipeYNegative) => true,
      _ => false,
    }
  }

  /// Check if this report contains a swipe-hold gesture (any direction).
  pub fn is_swipe_hold_gesture(&self) -> bool {
    match self.gesture {
      Some(Gesture::SwipeHoldXPositive)
      | Some(Gesture::SwipeHoldXNegative)
      | Some(Gesture::SwipeHoldYPositive)
      | Some(Gesture::SwipeHoldYNegative) => true,
      _ => false,
    }
  }

  /// Get the primary direction of a swipe gesture, if applicable.
  /// Returns `None` for non-swipe gestures.
  pub fn swipe_direction(&self) -> Option<SwipeDirection> {
    match self.gesture {
      Some(Gesture::SwipeXPositive) | Some(Gesture::SwipeHoldXPositive) => Some(SwipeDirection::Right),
      Some(Gesture::SwipeXNegative) | Some(Gesture::SwipeHoldXNegative) => Some(SwipeDirection::Left),
      Some(Gesture::SwipeYPositive) | Some(Gesture::SwipeHoldYPositive) => Some(SwipeDirection::Up),
      Some(Gesture::SwipeYNegative) | Some(Gesture::SwipeHoldYNegative) => Some(SwipeDirection::Down),
      _ => None,
    }
  }
}

/// Cardinal directions for swipe gestures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum SwipeDirection {
  /// Swipe toward positive X (right)
  Right,
  /// Swipe toward negative X (left)
  Left,
  /// Swipe toward positive Y (up)
  Up,
  /// Swipe toward negative Y (down)
  Down,
}

impl SwipeDirection {
  /// Returns `true` if this is a horizontal swipe direction.
  pub const fn is_horizontal(self) -> bool {
    matches!(self, SwipeDirection::Left | SwipeDirection::Right)
  }

  /// Returns `true` if this is a vertical swipe direction.
  pub const fn is_vertical(self) -> bool {
    matches!(self, SwipeDirection::Up | SwipeDirection::Down)
  }

  /// Returns the opposite direction.
  pub const fn opposite(self) -> Self {
    match self {
      SwipeDirection::Right => SwipeDirection::Left,
      SwipeDirection::Left => SwipeDirection::Right,
      SwipeDirection::Up => SwipeDirection::Down,
      SwipeDirection::Down => SwipeDirection::Up,
    }
  }

  /// Returns a human-readable string representation.
  pub const fn as_str(self) -> &'static str {
    match self {
      SwipeDirection::Right => "right",
      SwipeDirection::Left => "left",
      SwipeDirection::Up => "up",
      SwipeDirection::Down => "down",
    }
  }
}

/// Ergonomic façade on top of [`Iqs7211e`] that turns raw gestures and finger snapshots into
/// higher level touch events.
pub struct Touchpad<I, RDY> {
  controller: Iqs7211e<I, RDY>,
  previous: (Finger, Finger),
}

impl<I, RDY> Touchpad<I, RDY> {
  /// Create a new touchpad interface wrapping the given controller.
  pub fn new(controller: Iqs7211e<I, RDY>) -> Self {
    Self { controller, previous: (Finger::default(), Finger::default()) }
  }

  /// Consume the touchpad and return the underlying controller.
  pub fn into_inner(self) -> Iqs7211e<I, RDY> {
    self.controller
  }

  /// Get a mutable reference to the underlying controller.
  ///
  /// This provides access to low-level controller operations that may not
  /// be exposed through the high-level touchpad interface.
  pub fn controller(&mut self) -> &mut Iqs7211e<I, RDY> {
    &mut self.controller
  }

  /// Get an immutable reference to the underlying controller.
  pub fn controller_ref(&self) -> &Iqs7211e<I, RDY> {
    &self.controller
  }
}

impl<I, E, RDY> Touchpad<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Wait for the next hardware event and convert it into a [`Frame`].
  ///
  /// This is the primary method for receiving touch events. It blocks until
  /// the hardware signals a new event, then processes the raw data into a
  /// high-level touch report containing:
  ///
  /// - Touch contact changes (start/move/end events)
  /// - Current snapshot of all active touches
  /// - Detected gestures
  /// - Hardware status information
  pub async fn next_frame(&mut self) -> Result<Frame, Error<E>> {
    let report = self.controller.read_report().await?;
    let (contacts, snapshot) = build_contacts(self.previous, report);

    self.previous = report.fingers;

    Ok(Frame::new(report.info, report.gesture, contacts, snapshot))
  }

  /// Create an event stream that yields touch reports.
  ///
  /// This provides a convenient iterator-like interface for processing
  /// touch events in a loop. Each call to `next()` on the returned stream
  /// will wait for and return the next touch report.
  ///
  /// # Example
  ///
  /// ```no_run
  /// # use embedded_hal_async::{digital::Wait, i2c::{I2c, SevenBitAddress}};
  /// # use iqs7211e::Touchpad;
  /// # async fn example<I2C, RDY, E>(mut touchpad: Touchpad<I2C, RDY>) -> Result<(), iqs7211e::Error<E>>
  /// # where I2C: I2c<SevenBitAddress, Error = E>, RDY: Wait
  /// # {
  /// let mut stream = touchpad.stream();
  /// while let Some(report) = stream.next().await? {
  ///     // Process the touch report
  ///     println!("Changes: {}", report.snapshot().count());
  /// }
  /// # Ok(())
  /// # }
  /// ```
  pub fn stream(&mut self) -> Stream<'_, I, RDY> {
    Stream { touchpad: self }
  }
}

/// A stream of touch events from a touchpad.
///
/// This provides an iterator-like interface for processing touch events.
/// Create one using [`Touchpad::stream`].
pub struct Stream<'a, I, RDY> {
  touchpad: &'a mut Touchpad<I, RDY>,
}

impl<'a, I, E, RDY> Stream<'a, I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Get the next touch report from the stream.
  ///
  /// This blocks until a touch event occurs and returns the corresponding
  /// report. Returns `None` only if the stream is closed (which doesn't
  /// happen in the current implementation).
  pub async fn next(&mut self) -> Result<Option<Frame>, Error<E>> {
    Ok(Some(self.touchpad.next_frame().await?))
  }
}

fn build_contacts(previous: (Finger, Finger), report: Report) -> (Changes, State) {
  let new_fingers = report.fingers;
  let primary_contact = classify_transition(ContactSlot::Primary, previous.0, new_fingers.0);
  let secondary_contact = classify_transition(ContactSlot::Secondary, previous.1, new_fingers.1);
  let state = State::new(
    if new_fingers.0.is_present() {
      Some(new_fingers.0)
    } else {
      None
    },
    if new_fingers.1.is_present() {
      Some(new_fingers.1)
    } else {
      None
    },
  );
  (Changes::new(primary_contact, secondary_contact), state)
}

fn classify_transition(slot: ContactSlot, previous: Finger, current: Finger) -> Option<Touch> {
  match (previous.is_present(), current.is_present()) {
    (false, false) => None,
    (false, true) => Some(Touch::new(slot, TouchPhase::Start, current)),
    (true, false) => Some(Touch::new(slot, TouchPhase::End, previous)),
    (true, true) => {
      if previous != current {
        Some(Touch::new(slot, TouchPhase::Move, current))
      } else {
        None
      }
    }
  }
}

/// Utility functions for common touchpad operations and gesture analysis.
pub mod utils {
  use super::*;

  /// Classify the primary direction of movement between two contact points.
  ///
  /// Returns the dominant direction based on which axis has the larger
  /// displacement. Useful for implementing directional gesture recognition.
  pub fn movement_direction(from: Finger, to: Finger) -> SwipeDirection {
    let dx = if to.x > from.x { to.x - from.x } else { from.x - to.x };
    let dy = if to.y > from.y { to.y - from.y } else { from.y - to.y };

    if dx > dy {
      if to.x > from.x {
        SwipeDirection::Right
      } else {
        SwipeDirection::Left
      }
    } else {
      if to.y > from.y {
        SwipeDirection::Up
      } else {
        SwipeDirection::Down
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn classify_start() {
    let prev = Finger::absent();
    let current = Finger::new(10, 20, 30, 40);
    let contact = classify_transition(ContactSlot::Primary, prev, current).expect("start contact");
    assert_eq!(contact.phase, TouchPhase::Start);
    assert!(contact.is_primary());
    assert!(contact.phase.is_start());
    assert_eq!(contact.point.x, 10);
  }

  #[test]
  fn classify_move_requires_change() {
    let finger = Finger::new(10, 20, 30, 40);
    assert!(classify_transition(ContactSlot::Primary, finger, finger).is_none());

    let moved = Finger::new(11, 20, 30, 40);
    let contact = classify_transition(ContactSlot::Primary, finger, moved).expect("move contact");
    assert_eq!(contact.phase, TouchPhase::Move);
    assert!(contact.phase.is_move());
    assert_eq!(contact.point.x, 11);
  }

  #[test]
  fn classify_end_uses_previous_snapshot() {
    let prev = Finger::new(10, 20, 30, 40);
    let current = Finger::absent();
    let contact = classify_transition(ContactSlot::Secondary, prev, current).expect("end contact");
    assert_eq!(contact.phase, TouchPhase::End);
    assert!(contact.is_secondary());
    assert!(contact.phase.is_end());
    assert_eq!(contact.point.x, 10);
  }

  #[test]
  fn contact_events_iteration() {
    let primary = Touch::new(ContactSlot::Primary, TouchPhase::Start, Finger::new(1, 2, 3, 4));
    let events = Changes::new(Some(primary), None);
    let mut iter = events.iter();
    assert!(matches!(iter.next(), Some(c) if matches!(c.slot, ContactSlot::Primary)));
    assert!(iter.next().is_none());
  }

  #[test]
  fn touch_state_operations() {
    let p1 = Finger::new(10, 20, 100, 50);
    let p2 = Finger::new(30, 40, 200, 100);

    let state = State::new(Some(p1), Some(p2));

    assert_eq!(state.count(), 2);
    assert!(state.is_multi_touch());
    assert!(!state.is_empty());

    let centroid = state.centroid().expect("centroid");
    assert_eq!(centroid.x, 20); // (10 + 30) / 2
    assert_eq!(centroid.y, 30); // (20 + 40) / 2
    assert_eq!(centroid.strength, 150); // (100 + 200) / 2
  }
  #[test]
  fn contact_events_phase_filtering() {
    let start_contact = Touch::new(ContactSlot::Primary, TouchPhase::Start, Finger::new(10, 10, 100, 50));
    let end_contact = Touch::new(ContactSlot::Secondary, TouchPhase::End, Finger::new(20, 20, 150, 75));

    let events = Changes::new(Some(start_contact), Some(end_contact));

    assert!(events.has_starts());
    assert!(events.has_ends());
    assert!(!events.has_moves());

    let starts_count = events.contacts_with_phase(TouchPhase::Start).count();
    assert_eq!(starts_count, 1);

    let start_found = events.contacts_with_phase(TouchPhase::Start).next().unwrap();
    assert!(start_found.is_primary());
  }

  #[test]
  fn gesture_classification() {
    use crate::defs::{ChargeMode, InfoFlags};
    use crate::event::Gesture;

    let info_flags = InfoFlags {
      charge_mode: ChargeMode::Active,
      auto_tuning_error: false,
      re_auto_tuning_occurred: false,
      low_power_auto_tuning_error: false,
      low_power_re_auto_tuning_occurred: false,
      show_reset: false,
      num_fingers: 0,
      trackpad_movement: false,
      too_many_fingers: false,
      low_power_output: false,
    };

    let swipe_report =
      Frame::new(info_flags, Some(Gesture::SwipeXPositive), Changes::new(None, None), State::new(None, None));

    assert!(swipe_report.is_swipe_gesture());
    assert!(!swipe_report.is_tap_gesture());
    assert_eq!(swipe_report.swipe_direction(), Some(SwipeDirection::Right));

    let tap_report = Frame::new(info_flags, Some(Gesture::DoubleTap), Changes::new(None, None), State::new(None, None));

    assert!(tap_report.is_tap_gesture());
    assert!(!tap_report.is_swipe_gesture());
  }

  #[test]
  fn swipe_direction_properties() {
    assert!(SwipeDirection::Left.is_horizontal());
    assert!(SwipeDirection::Right.is_horizontal());
    assert!(SwipeDirection::Up.is_vertical());
    assert!(SwipeDirection::Down.is_vertical());

    assert_eq!(SwipeDirection::Left.opposite(), SwipeDirection::Right);
    assert_eq!(SwipeDirection::Up.opposite(), SwipeDirection::Down);
  }

  #[test]
  fn contact_slot_properties() {
    assert!(ContactSlot::Primary.is_primary());
    assert!(!ContactSlot::Primary.is_secondary());
    assert!(ContactSlot::Secondary.is_secondary());
    assert!(!ContactSlot::Secondary.is_primary());
  }

  #[test]
  fn session_detection() {
    use crate::defs::{ChargeMode, InfoFlags};

    let info_flags = InfoFlags {
      charge_mode: ChargeMode::Active,
      auto_tuning_error: false,
      re_auto_tuning_occurred: false,
      low_power_auto_tuning_error: false,
      low_power_re_auto_tuning_occurred: false,
      show_reset: false,
      num_fingers: 0,
      trackpad_movement: false,
      too_many_fingers: false,
      low_power_output: false,
    };

    // Session start: first touch begins
    let start_contact = Touch::new(ContactSlot::Primary, TouchPhase::Start, Finger::new(10, 10, 100, 50));
    let start_report = Frame::new(
      info_flags,
      None,
      Changes::new(Some(start_contact), None),
      State::new(Some(Finger::new(10, 10, 100, 50)), None),
    );

    assert!(start_report.is_session_start());
    assert!(!start_report.is_session_end());

    // Session end: last touch ends
    let end_contact = Touch::new(ContactSlot::Primary, TouchPhase::End, Finger::new(10, 10, 100, 50));
    let end_report = Frame::new(info_flags, None, Changes::new(Some(end_contact), None), State::new(None, None));

    assert!(!end_report.is_session_start());
    assert!(end_report.is_session_end());
  }
}
