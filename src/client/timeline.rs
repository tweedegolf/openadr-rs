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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
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
    data: rangemap::RangeMap<chrono::DateTime<chrono::Utc>, Value>,
}

impl Timeline {
    pub fn new(program: &ProgramContent, mut events: Vec<&EventContent>) -> Result<Self, ()> {
        let mut data = Self::default();

        events.sort_by_key(|e| e.priority);

        for event in events {
            // SPEC ASSUMPTION: At least one of the following `interval_period`s must be given on the program, on the event, or on the interval
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
                    None => *start..DateTime::<Utc>::MAX_UTC,
                };

                let value = Value {
                    randomize_start: randomize_start
                        .as_ref()
                        .map(|d| d.to_chrono_at_datetime(*start)),
                    value_map: interval.payloads.clone(),
                    priority: event.priority,
                };

                for (existing_range, existing) in data.data.overlapping(&range) {
                    if existing.priority == event.priority {
                        warn!(?existing_range, ?existing, new_range = ?range, new = ?value, "Overlapping ranges with equal priority");
                    }
                }
                data.data.insert(range, value);
            }
        }

        Ok(data)
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

    fn make_interval(range: Range<u32>, value: i64) -> EventInterval {
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

    fn make_vinterval(
        range: Range<u32>,
        value: i64,
        priority: Priority,
    ) -> (Range<DateTime<Utc>>, super::Value) {
        let start = DateTime::UNIX_EPOCH + Duration::hours(range.start.into());
        let end = DateTime::UNIX_EPOCH + Duration::hours(range.end.into());

        (
            start..end,
            super::Value {
                randomize_start: None,
                value_map: vec![EventValuesMap {
                    value_type: crate::wire::event::EventType::Price,
                    values: vec![Value::Integer(value)],
                }],
                priority,
            },
        )
    }

    #[test]
    fn priorities() {
        let program = ProgramContent::new("p");
        let program_id = ProgramId("p-id".into());
        let event = EventContent::new(program_id.clone(), vec![make_interval(0..10, 42)]);
        let prio_event = EventContent::new(program_id.clone(), vec![make_interval(5..15, 43)])
            .with_priority(Priority::MAX);

        let tl = Timeline::new(&program, vec![&prio_event, &event]).unwrap();

        assert_eq!(
            tl.data.into_iter().collect::<Vec<_>>(),
            vec![
                make_vinterval(0..5, 42, Priority::UNSPECIFIED),
                make_vinterval(5..15, 43, Priority::MAX)
            ]
        );
    }

    #[test]
    fn overlap_same_priority() {
        let program = ProgramContent::new("p");
        let program_id = ProgramId("p-id".into());
        let event1 = EventContent::new(program_id.clone(), vec![make_interval(0..10, 42)]);
        let event2 = EventContent::new(program_id.clone(), vec![make_interval(5..15, 43)]);

        let tl1 = Timeline::new(&program, vec![&event1, &event2]).unwrap();
        assert_eq!(
            tl1.data.into_iter().collect::<Vec<_>>(),
            vec![
                make_vinterval(0..5, 42, Priority::UNSPECIFIED),
                make_vinterval(5..15, 43, Priority::UNSPECIFIED),
            ]
        );

        let tl2 = Timeline::new(&program, vec![&event2, &event1]).unwrap();
        assert_eq!(
            tl2.data.into_iter().collect::<Vec<_>>(),
            vec![
                make_vinterval(0..10, 42, Priority::UNSPECIFIED),
                make_vinterval(10..15, 43, Priority::UNSPECIFIED),
            ]
        );
    }
}
