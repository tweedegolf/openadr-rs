#![allow(dead_code)]
use crate::{
    wire::{
        event::{EventValuesMap, Priority},
        interval::IntervalPeriod,
    },
    EventContent, ProgramContent,
};

use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub struct ValuedInterval {
    pub range: Range<chrono::DateTime<chrono::Utc>>,
    /// Relative priority of event. A lower number is a higher priority.
    pub priority: Priority,
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
    pub fn new() -> Self {
        Self::default()
    }

    // SPEC ASSUMPTION: At least one of the following `interval_period`s must be given on the program, on the event, or on the interval
    pub fn add_event_content(
        &mut self,
        program: &ProgramContent,
        event: &EventContent,
        range_of_interest: &Range<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), ()> {
        let default_period = event
            .interval_period
            .as_ref()
            .or(program.interval_period.as_ref());

        for interval in &event.intervals {
            // use the even't interval period when the interval doesn't specify one
            let period = interval
                .interval_period
                .as_ref()
                .or(default_period)
                .ok_or(())?;

            let IntervalPeriod {
                start,
                duration,
                randomize_start,
            } = period;

            let range = match duration {
                Some(duration) => *start..*start + duration.to_chrono_at_datetime(*start),
                None => *start..range_of_interest.end,
            };

            let Some(RangeIntersection { middle: range, .. }) =
                RangeIntersection::new(&range, range_of_interest)
            else {
                continue;
            };

            self.insert(ValuedInterval {
                range,
                priority: event.priority,
                value_map: interval.payloads.clone(),
                randomize_start: randomize_start
                    .as_ref()
                    .map(|d| d.to_chrono_at_datetime(*start)),
            });
        }

        Ok(())
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
                intersection,
            } => {
                // replace the overlapping section if the priority warrants it
                if element.priority > self.data[index].priority
                    && self.data[index].range == intersection.middle
                {
                    self.data[index] = ValuedInterval {
                        range: intersection.middle,
                        randomize_start: match intersection.left {
                            Some(_) => None, // to_the_left has the randomize_start already
                            None => self.data[index].randomize_start,
                        },
                        ..element.clone()
                    };
                }

                // then insert the non-overlapping left section (if any)
                if let Some(range) = intersection.left {
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
                if let Some(range) = intersection.right {
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
        intersection: RangeIntersection<T>,
    },
}

impl<T: Ord + Copy> Action<T> {
    fn insert<'a, I>(it: I, element: Range<T>) -> Self
    where
        I: ExactSizeIterator<Item = &'a Range<T>>,
        T: 'a,
    {
        let len = it.len();
        for (i, range) in it.enumerate() {
            if let Some(intersection) = RangeIntersection::new(&element, range) {
                return Action::HandleOverlapAt {
                    index: i,
                    intersection,
                };
            } else if element.start >= range.end {
                continue;
            } else if element.end <= range.start {
                return Action::InsertAt(i);
            } else {
                unreachable!()
            }
        }

        Action::InsertAt(len)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RangeIntersection<T> {
    left: Option<Range<T>>,
    middle: Range<T>,
    right: Option<Range<T>>,
}

impl<T: Ord + Copy> RangeIntersection<T> {
    pub fn new(element: &Range<T>, range: &Range<T>) -> Option<Self> {
        if element.start >= range.end || element.end <= range.start {
            None
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

            Some(Self {
                left,
                middle,
                right,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use chrono::{DateTime, Duration, Utc};

    use crate::{
        wire::{event::EventInterval, values_map::Value},
        ProgramId,
    };

    use super::*;

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
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
                intersection: RangeIntersection {
                    left: None,
                    middle: 3..5,
                    right: Some(5..10),
                }
            }
        );

        // overlap right
        assert_eq!(
            Action::insert([0..5, 10..15].iter(), 5..12),
            Action::HandleOverlapAt {
                index: 1,
                intersection: RangeIntersection {
                    left: Some(5..10),
                    middle: 10..12,
                    right: None,
                }
            }
        );

        // overlap both
        assert_eq!(
            Action::insert([5..10].iter(), 0..15),
            Action::HandleOverlapAt {
                index: 0,
                intersection: RangeIntersection {
                    left: Some(0..5),
                    middle: 5..10,
                    right: Some(10..15),
                }
            }
        );
    }

    #[test]
    fn priorities() {
        fn make_interval(start: u32, duration: u32, value: i64) -> EventInterval {
            EventInterval {
                id: start as _,
                interval_period: Some(IntervalPeriod {
                    start: DateTime::UNIX_EPOCH + Duration::hours(start.into()),
                    duration: Some(crate::wire::Duration::hours(duration as _)),
                    randomize_start: None,
                }),
                payloads: vec![EventValuesMap {
                    value_type: crate::wire::event::EventType::Price,
                    values: vec![Value::Integer(value)],
                }],
            }
        }

        fn make_vinterval(start: u32, end: u32, value: i64, priority: Priority) -> ValuedInterval {
            let start = DateTime::UNIX_EPOCH + Duration::hours(start.into());
            let end = DateTime::UNIX_EPOCH + Duration::hours(end.into());

            ValuedInterval {
                range: start..end,
                priority,
                randomize_start: None,
                value_map: vec![EventValuesMap {
                    value_type: crate::wire::event::EventType::Price,
                    values: vec![Value::Integer(value)],
                }],
            }
        }

        let total_range = DateTime::<Utc>::MIN_UTC..DateTime::<Utc>::MAX_UTC;

        let program = ProgramContent::new("p");
        let program_id = ProgramId("p-id".into());
        let event = EventContent::new(program_id.clone(), vec![make_interval(0, 10, 42)]);
        let prio_event = EventContent::new(program_id.clone(), vec![make_interval(5, 10, 43)])
            .with_priority(Priority::MAX);

        let mut tl = Timeline::new();
        tl.add_event_content(&program, &event, &total_range)
            .unwrap();
        tl.add_event_content(&program, &prio_event, &total_range)
            .unwrap();

        assert_eq!(
            tl.data,
            vec![
                make_vinterval(0, 5, 42, Priority::UNSPECIFIED),
                make_vinterval(5, 15, 43, Priority::MAX)
            ]
        );
    }
}
