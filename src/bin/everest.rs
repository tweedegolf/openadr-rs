use chrono::{DateTime, Utc};
use openadr::{
    wire::{
        event::{EventType, EventValuesMap},
        values_map::Value,
    },
    Target, Timeline,
};
use std::future::Future;
use std::{
    error::Error,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

async fn wait_for_next_start(clock: &impl Clock, timeline: &Timeline) {
    let now = clock.now();

    let Some(next) = timeline.next_update(&now) else {
        return std::future::pending().await; // Wait forever
    };

    match (next - now).to_std() {
        Err(_) => return,
        Ok(delta) => tokio::time::sleep(delta).await,
    }
}

trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

struct ChronoClock;

impl Clock for ChronoClock {
    fn now(&self) -> DateTime<Utc> {
        chrono::Utc::now()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    run(ChronoClock).await
}

async fn run(clock: impl Clock) -> Result<(), Box<dyn Error>> {
    // channel used to send new timelines
    let (sender, receiver) = tokio::sync::mpsc::channel(1);

    let client = openadr::Client::new("http://localhost:3000/".try_into()?);
    let mut program = client.get_program(Target::Program("name")).await?;

    let poll_interval = Duration::from_secs(30);

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(poll_interval).await;

            let timeline = program.get_timeline().await?;

            sender.send(timeline).await.unwrap();
        }

        #[allow(unreachable_code)]
        Ok::<(), openadr::Error>(())
    });

    let (output_sender, output_receiver) = tokio::sync::mpsc::channel(1);

    tokio::spawn(update_listener(ChronoClock, receiver, output_sender));

    Ok(())
}

async fn update_listener(
    clock: impl Clock,
    mut receiver: Receiver<Timeline>,
    mut sender: Sender<EnforcedLimits>,
) {
    let mut timeline = Timeline::new();
    loop {
        select! {
            result = receiver.recv() => {
                match result {
                    None => break, // sender was dropped
                    Some(new_timeline) => timeline = new_timeline,
                }
            }
            () = wait_for_next_start(&clock, &timeline) => {
                //  fall through
            }
        };

        let now = clock.now();

        let Some(current) = timeline.at_datetime(&now) else {
            continue;
        };

        let mut schedule = Vec::new();
        let mut valid_until = None;

        for (range, interval) in timeline.iter() {
            if range.end < now {
                continue;
            }

            valid_until = Ord::max(valid_until, Some(range.end.clone()));

            let entry = ScheduleResEntry {
                timestamp: range.start,
                limits_to_root: LimitsRes::try_from_event_values(interval.value_map).unwrap(),
            };

            schedule.push(entry);
        }

        let enforced_limits = EnforcedLimits {
            uuid: Uuid::new_v4().to_string(),
            valid_until: valid_until.unwrap(),
            limits_root_side: LimitsRes::try_from_event_values(current.1.value_map).unwrap(),
            schedule,
        };

        let Ok(()) = sender.send(enforced_limits).await else {
            break;
        };
    }
}

// https://github.com/tdittr/everest-core/blob/openadr/types/energy.yaml#L213
#[derive(Debug, Clone, PartialEq)]
struct EnforcedLimits {
    uuid: String,
    valid_until: chrono::DateTime<chrono::Utc>,
    limits_root_side: LimitsRes,
    schedule: Vec<ScheduleResEntry>,
}

// https://github.com/tdittr/everest-core/blob/openadr/types/energy.yaml#L125
#[derive(Debug, Clone, Copy, PartialEq)]
struct ScheduleResEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    limits_to_root: LimitsRes,
}

// https://github.com/tdittr/everest-core/blob/openadr/types/energy.yaml#L46
#[derive(Debug, Clone, Copy, PartialEq)]
struct LimitsRes {
    // NOTE: that W is uppercase if we ever need to serialize this!
    total_power_w: f64,
}

impl LimitsRes {
    fn try_from_event_values(values: &[EventValuesMap]) -> Option<Self> {
        for EventValuesMap { value_type, values } in values {
            if let EventType::ImportCapacityLimit = value_type {
                if let [Value::Number(value)] = &values[..] {
                    return Some(Self {
                        total_power_w: *value,
                    });
                } else {
                    panic!("invalid values {:?}", values);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use openadr::wire::event::EventInterval;
    use openadr::wire::interval::IntervalPeriod;
    use openadr::{EventContent, ProgramContent, ProgramId};
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;
    use tracing_subscriber::fmt::time;

    struct TestingClock(AtomicU64);

    impl Clock for Arc<TestingClock> {
        fn now(&self) -> DateTime<Utc> {
            let millis = self.0.load(Ordering::Relaxed);
            chrono::DateTime::<Utc>::from_timestamp_millis(millis as i64).unwrap()
        }
    }

    impl TestingClock {
        pub fn new(now: DateTime<Utc>) -> Arc<Self> {
            Arc::new(Self(AtomicU64::new(
                now.timestamp_millis().try_into().unwrap(),
            )))
        }

        async fn advance(&self, duration: std::time::Duration) {
            self.0
                .fetch_add(duration.as_millis().try_into().unwrap(), Ordering::Relaxed);
            tokio::time::advance(duration).await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn foobar() {
        let clock = TestingClock::new("1979-10-12T09:42:00Z".parse().unwrap());
        let past = tokio::time::Instant::now();

        let (input_sender, input_receiver) = tokio::sync::mpsc::channel(1);
        let (output_sender, mut output_receiver) = tokio::sync::mpsc::channel(1);

        let handle = tokio::spawn(update_listener(
            Arc::clone(&clock),
            input_receiver,
            output_sender,
        ));
        assert!(!handle.is_finished());
        assert!(!output_receiver.is_closed());
        assert!(output_receiver.is_empty());

        assert_eq!(past, tokio::time::Instant::now());
        clock.advance(Duration::from_secs(60)).await;
        assert!(output_receiver.is_empty());

        let timeline = create_timeline(vec![
            ("1979-10-12T09:00:00Z", 42.0),
            ("1979-10-12T10:00:00Z", 21.0),
        ]);
        input_sender.send(timeline).await.unwrap();

        let output = output_receiver.recv().await.unwrap();
        assert_eq!(output.limits_root_side.total_power_w, 42.0);
        assert_eq!(
            output.schedule,
            vec![
                ScheduleResEntry {
                    timestamp: "1979-10-12T09:00:00Z".parse().unwrap(),
                    limits_to_root: LimitsRes {
                        total_power_w: 42.0
                    }
                },
                ScheduleResEntry {
                    timestamp: "1979-10-12T10:00:00Z".parse().unwrap(),
                    limits_to_root: LimitsRes {
                        total_power_w: 21.0
                    }
                }
            ]
        );

        clock.advance(Duration::from_secs(60 * 60)).await;
        let output = output_receiver.recv().await.unwrap();
        assert_eq!(output.limits_root_side.total_power_w, 21.0);
        assert_eq!(
            output.schedule,
            vec![ScheduleResEntry {
                timestamp: "1979-10-12T10:00:00Z".parse().unwrap(),
                limits_to_root: LimitsRes {
                    total_power_w: 21.0
                }
            }]
        );
    }

    fn create_timeline(entries: Vec<(&str, f64)>) -> Timeline {
        let intervals = entries
            .into_iter()
            .map(|(start_time, value)| EventInterval {
                id: 0,
                interval_period: Some(IntervalPeriod::new(start_time.parse().unwrap())),
                payloads: vec![EventValuesMap {
                    value_type: EventType::ImportCapacityLimit,
                    values: vec![Value::Number(value)],
                }],
            })
            .collect();

        let program = ProgramContent::new("Limits for Arthur Dent");
        let event = EventContent::new(ProgramId::new("ad").unwrap(), intervals);
        let events = vec![&event];
        let timeline = Timeline::from_events(&program, events).unwrap();
        timeline
    }
}
