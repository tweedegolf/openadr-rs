use crate::{
    wire::{event::EventValuesMap, interval::IntervalPeriod},
    EventContent,
};

use std::ops::Range;

#[derive(Debug, Clone)]
pub struct ValuedInterval {
    pub range: Range<chrono::DateTime<chrono::Utc>>,
    /// Relative priority of event. A lower number is a higher priority.
    pub priority: Option<u32>,
    /// Indicates a randomization time that may be applied to start.
    pub randomize_start: Option<chrono::Duration>,
    /// The actual values that are active during this interval
    pub value_map: Vec<EventValuesMap>,
}

/// A sequence of ordered, non-overlapping intervals and associated values.
///
/// Intervals are sorted by their timestamp. The intervals will not overlap, but there may be gaps
/// between intervals.
#[allow(unused)]
#[derive(Default)]
pub struct Timeline {
    data: Vec<ValuedInterval>,
}

impl Timeline {
    fn from_event_content(event: &EventContent) -> Self {
        let mut this = Self::default();

        let period = &event.interval_period;

        for interval in &event.intervals {
            // use the even't interval period when the interval doesn't specify one
            match interval.interval_period.as_ref().or(period.as_ref()) {
                Some(IntervalPeriod {
                    start,
                    duration,
                    randomize_start,
                }) => {
                    let range = match duration {
                        Some(duration) => *start..*start + duration.0.to_chrono_at_datetime(*start),
                        None => *start..chrono::DateTime::<chrono::Utc>::MAX_UTC,
                    };

                    this.insert(ValuedInterval {
                        range,
                        priority: event.priority,
                        value_map: interval.payloads.clone(),
                        // NOTE: this will silently fail when randomize_start includes years/months
                        randomize_start: randomize_start
                            .as_ref()
                            .map(|d| d.0.to_chrono_at_datetime(*start)),
                    });
                }
                None => {
                    panic!("event without an interval")
                }
            }
        }

        this
    }

    #[allow(unused)]
    pub fn insert(&mut self, element: ValuedInterval) {
        let action = Action::insert(
            self.data.iter().map(|int| &int.range),
            element.range.clone(),
        );

        match action {
            Action::InsertAt(i) => {
                // `.insert(self.data.len(), element)` is equivalent to a `.push(element)`
                self.data.insert(i, element);
            }
            Action::HandleOverlapAt {
                index,
                left,
                middle,
                right,
            } => {
                // replace the overlapping section if the priority warrants it
                if element.priority < self.data[index].priority {
                    self.data[index] = ValuedInterval {
                        range: middle,
                        randomize_start: match left {
                            Some(_) => None, // to_the_left has the randomize_start already
                            None => self.data[index].randomize_start,
                        },
                        ..element.clone()
                    };
                }

                // then insert the non-overlapping left section (if any)
                if let Some(range) = left {
                    self.data.insert(
                        index,
                        ValuedInterval {
                            range,
                            ..element.clone()
                        },
                    )
                };

                // then insert the right section. This won't overlap with `self.data[index]`, but
                // may overlap with `self.data[index + 1]`.
                if let Some(range) = right {
                    self.insert(ValuedInterval {
                        range,
                        randomize_start: None,
                        ..element.clone()
                    })
                };
            }
        }
    }
}

// helper for how to insert a new interval into an existing sequence of intervals
#[derive(Debug, PartialEq, Eq)]
enum Action<T> {
    InsertAt(usize),
    HandleOverlapAt {
        index: usize,
        left: Option<Range<T>>,
        middle: Range<T>,
        right: Option<Range<T>>,
    },
}

impl<T: Ord + Copy> Action<T> {
    fn insert<'a, I>(it: I, element: Range<T>) -> Self
    where
        I: Iterator<Item = &'a Range<T>>,
        T: 'a,
    {
        let mut i = 0;
        for range in it {
            if element.start >= range.end {
                i += 1;
                continue;
            } else if element.end <= range.start {
                return Action::InsertAt(i);
            } else {
                let left = if element.start < range.start {
                    Some(element.start..range.start)
                } else {
                    None
                };

                let middle = Ord::max(element.start, range.start)..Ord::min(element.end, range.end);

                let right = if element.end > range.end {
                    Some(range.end..element.end)
                } else {
                    None
                };

                return Action::HandleOverlapAt {
                    index: i,
                    left,
                    middle,
                    right,
                };
            }
        }

        Action::InsertAt(i)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn find_overlap() {
        assert_eq!(Action::insert([0..5].iter(), 5..10), Action::InsertAt(1));
        assert_eq!(
            Action::insert([0..5, 10..15].iter(), 5..10),
            Action::InsertAt(1)
        );

        // overlap left
        assert_eq!(
            Action::insert([0..5, 10..15].iter(), 3..10),
            Action::HandleOverlapAt {
                index: 0,
                left: None,
                middle: 3..5,
                right: Some(5..10),
            }
        );

        // overlap right
        assert_eq!(
            Action::insert([0..5, 10..15].iter(), 5..12),
            Action::HandleOverlapAt {
                index: 1,
                left: Some(5..10),
                middle: 10..12,
                right: None,
            }
        );

        // overlap both
        assert_eq!(
            Action::insert([5..10].iter(), 0..15),
            Action::HandleOverlapAt {
                index: 0,
                left: Some(0..5),
                middle: 5..10,
                right: Some(10..15),
            }
        );
    }
}
