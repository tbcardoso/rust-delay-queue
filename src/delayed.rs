use std::time::{Duration, Instant};

/// A value that is delayed until some `Instant`.
///
/// The `DelayQueue` only accepts values that implement this trait.
/// In most situations you do not need to implement this trait yourself. You can use the helper
/// struct `Delay`.
pub trait Delayed {
    /// Returns the `Instant` until which this value is delayed.
    fn delayed_until(&self) -> Instant;
}

/// Wraps a value that should be delayed.
///
/// Implements `Delayed` and `Eq`. Two `Delay` objects are equal iff their wrapped `value`s are
/// equal and they are delayed until the same `Instant`.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use delay_queue::{Delay, Delayed};
/// use std::time::{Duration, Instant};
///
/// let delayed_one_hour = Delay::for_duration(123, Duration::from_secs(3600));
/// let delayed_now = Delay::until_instant("abc", Instant::now());
///
/// assert!(delayed_one_hour.delayed_until() > delayed_now.delayed_until());
/// assert_eq!(delayed_one_hour.value, 123);
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Delay<T> {
    /// The value that is delayed.
    pub value: T,

    /// The `Instant` until which `value` is delayed.
    until: Instant,
}

impl<T> Delay<T> {
    /// Creates a new `Delay` holding `value` and that is delayed until the given `Instant`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use delay_queue::Delay;
    /// use std::time::Instant;
    ///
    /// let delayed_now = Delay::until_instant("abc", Instant::now());
    /// ```
    pub fn until_instant(value: T, until: Instant) -> Delay<T> {
        Delay { value, until }
    }

    /// Creates a new `Delay` holding `value` and that is delayed until the given `Duration` has
    /// elapsed.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use delay_queue::Delay;
    /// use std::time::Duration;
    ///
    /// let delayed_one_hour = Delay::for_duration("abc", Duration::from_secs(3600));
    /// ```
    pub fn for_duration(value: T, duration: Duration) -> Delay<T> {
        Delay::until_instant(value, Instant::now() + duration)
    }
}

impl<T> Delayed for Delay<T> {
    fn delayed_until(&self) -> Instant {
        self.until
    }
}

impl<T: Default> Default for Delay<T> {
    fn default() -> Delay<T> {
        Delay {
            value: Default::default(),
            until: Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};
    use super::{Delay, Delayed};

    #[test]
    fn compare_until() {
        let delayed_one_hour = Delay::for_duration(123, Duration::from_secs(3600));
        let delayed_now = Delay::until_instant("abc", Instant::now());

        assert!(delayed_one_hour.delayed_until() > delayed_now.delayed_until());
    }

    #[test]
    fn correct_value() {
        let delayed_one_hour = Delay::for_duration(123, Duration::from_secs(3600));
        let delayed_now = Delay::until_instant("abc", Instant::now());

        assert_eq!(delayed_one_hour.value, 123);
        assert_eq!(delayed_now.value, "abc");
    }
}
