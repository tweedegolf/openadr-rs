#![allow(dead_code)]
use chrono::{DateTime, Utc};
use tracing::warn;

use crate::{
    wire::{
        event::{EventValuesMap, Priority},
        interval::IntervalPeriod,
    },
    EventContent, ProgramContent,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interval {
    /// Relative priority of event
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
    data: rangemap::RangeMap<chrono::DateTime<chrono::Utc>, Interval>,
}

impl Timeline {
    pub fn new(program: &ProgramContent, mut events: Vec<&EventContent>) -> Result<Self, ()> {
        let mut data = Self::default();

        events.sort_by_key(|e| e.priority);

        for event in events {
            // SPEC ASSUMPTION: At least one of the following `interval_period`s must be given on the program,
            // on the event, or on the interval
            let default_period = event
                .interval_period
                .as_ref()
                .or(program.interval_period.as_ref());

            for event_interval in &event.intervals {
                // use the even't interval period when the interval doesn't specify one
                let period = event_interval
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
                    None => *start..DateTime::<Utc>::MAX_UTC,
                };

                let interval = Interval {
                    randomize_start: randomize_start
                        .as_ref()
                        .map(|d| d.to_chrono_at_datetime(*start)),
                    value_map: event_interval.payloads.clone(),
                    priority: event.priority,
                };

                for (existing_range, existing) in data.data.overlapping(&range) {
                    if existing.priority == event.priority {
                        warn!(?existing_range, ?existing, new_range = ?range, new = ?interval, "Overlapping ranges with equal priority");
                    }
                }

                data.data.insert(range, interval);
            }
        }

        Ok(data)
    }
}

#[cfg(test)]
mod test {
    use std::ops::Range;

    use chrono::{DateTime, Duration, Utc};

    use crate::{
        wire::{event::EventInterval, values_map::Value},
        ProgramId,
    };

    use super::*;

    fn test_program_id() -> ProgramId {
        ProgramId("test-program-id".into())
    }

    fn test_event_content(range: Range<u32>, value: i64) -> EventContent {
        EventContent::new(
            test_program_id(),
            vec![event_interval_with_value(range, value)],
        )
    }

    fn event_interval_with_value(range: Range<u32>, value: i64) -> EventInterval {
        EventInterval {
            id: range.start as _,
            interval_period: Some(IntervalPeriod {
                start: DateTime::UNIX_EPOCH + Duration::hours(range.start.into()),
                duration: Some(crate::wire::Duration::hours((range.end - range.start) as _)),
                randomize_start: None,
            }),
            payloads: vec![EventValuesMap {
                value_type: crate::wire::event::EventType::Price,
                values: vec![Value::Integer(value)],
            }],
        }
    }

    fn interval_with_value(
        range: Range<u32>,
        value: i64,
        priority: Priority,
    ) -> (Range<DateTime<Utc>>, super::Interval) {
        let start = DateTime::UNIX_EPOCH + Duration::hours(range.start.into());
        let end = DateTime::UNIX_EPOCH + Duration::hours(range.end.into());

        (
            start..end,
            super::Interval {
                randomize_start: None,
                value_map: vec![EventValuesMap {
                    value_type: crate::wire::event::EventType::Price,
                    values: vec![Value::Integer(value)],
                }],
                priority,
            },
        )
    }

    // the spec does not specify the behavior when two intervals with the same priority overlap.
    // Our current implementation uses `RangeMap`, and its behavior is to overwrite the existing
    // range with a new one. In other words: the event that is inserted last wins.
    #[test]
    fn overlap_same_priority() {
        let program = ProgramContent::new("p");

        let event1 = test_event_content(0..10, 42);
        let event2 = test_event_content(5..15, 43);

        // first come, last serve
        let tl1 = Timeline::new(&program, vec![&event1, &event2]).unwrap();
        assert_eq!(
            tl1.data.into_iter().collect::<Vec<_>>(),
            vec![
                interval_with_value(0..5, 42, Priority::UNSPECIFIED),
                interval_with_value(5..15, 43, Priority::UNSPECIFIED),
            ]
        );

        // first come, last serve
        let tl2 = Timeline::new(&program, vec![&event2, &event1]).unwrap();
        assert_eq!(
            tl2.data.into_iter().collect::<Vec<_>>(),
            vec![
                interval_with_value(0..10, 42, Priority::UNSPECIFIED),
                interval_with_value(10..15, 43, Priority::UNSPECIFIED),
            ]
        );
    }

    #[test]
    fn overlap_lower_priority() {
        let event1 = test_event_content(0..10, 42).with_priority(Priority::new(1));
        let event2 = test_event_content(5..15, 43).with_priority(Priority::new(2));

        let tl = Timeline::new(&ProgramContent::new("p"), vec![&event1, &event2]).unwrap();
        assert_eq!(
            tl.data.into_iter().collect::<Vec<_>>(),
            vec![
                interval_with_value(0..10, 42, Priority::new(1)),
                interval_with_value(10..15, 43, Priority::new(2)),
            ],
            "a lower priority event MUST NOT overwrite a higher priority one",
        );

        let tl = Timeline::new(&ProgramContent::new("p"), vec![&event2, &event1]).unwrap();
        assert_eq!(
            tl.data.into_iter().collect::<Vec<_>>(),
            vec![
                interval_with_value(0..10, 42, Priority::new(1)),
                interval_with_value(10..15, 43, Priority::new(2)),
            ],
            "a lower priority event MUST NOT overwrite a higher priority one",
        );
    }

    #[test]
    fn overlap_higher_priority() {
        let event1 = test_event_content(0..10, 42).with_priority(Priority::new(2));
        let event2 = test_event_content(5..15, 43).with_priority(Priority::new(1));

        let tl = Timeline::new(&ProgramContent::new("p"), vec![&event1, &event2]).unwrap();
        assert_eq!(
            tl.data.into_iter().collect::<Vec<_>>(),
            vec![
                interval_with_value(0..5, 42, Priority::new(2)),
                interval_with_value(5..15, 43, Priority::new(1)),
            ],
            "a higher priority event MUST overwrite a lower priority one",
        );

        let tl = Timeline::new(&ProgramContent::new("p"), vec![&event2, &event1]).unwrap();
        assert_eq!(
            tl.data.into_iter().collect::<Vec<_>>(),
            vec![
                interval_with_value(0..5, 42, Priority::new(2)),
                interval_with_value(5..15, 43, Priority::new(1)),
            ],
            "a higher priority event MUST overwrite a lower priority one",
        );
    }
}
