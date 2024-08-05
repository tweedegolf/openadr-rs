use std::{error::Error, time::Duration};

use openadr::{
    wire::{
        event::{EventType, EventValuesMap},
        values_map::Value,
    },
    EventClient, ProgramContent, Target, Timeline,
};
use tokio::select;
use uuid::Uuid;

async fn wait_for_next_start(entries: &[ScheduleResEntry]) -> Option<ScheduleResEntry> {
    match entries {
        [] => {
            // just wait for a timeline to come in
            std::future::pending::<()>().await;
            None // unreachable in practice
        }
        [.., last] => {
            let delta = last.timestamp - chrono::Utc::now();

            match delta.to_std() {
                Ok(delta) => {
                    tokio::time::sleep(delta).await;
                    Some(*last)
                }
                Err(_) => {}
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // channel used to send new timelines
    let (sender, mut receiver) = tokio::sync::watch::channel(None);

    let client = openadr::Client::new("http://localhost:3000/".try_into()?);
    let program = client.get_program(Target::Program("name")).await?;
    let program_content = program.data().clone();

    let poll_interval = Duration::from_secs(30);

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(poll_interval).await;

            let events = program.get_all_events().await?;

            sender.send(Some(events)).unwrap();
        }

        #[allow(unreachable_code)]
        Ok::<(), openadr::Error>(())
    });

    tokio::spawn(async move {
        let mut job_stack = Vec::<ScheduleResEntry>::new();
        let mut enforced_limits = None;

        loop {
            select! {
                result = receiver.changed() => {
                    match result {
                        Err(_) => break, // sender was dropped
                        Ok(()) => {
                            let events = receiver.borrow_and_update();
                            let Some(events) = events.as_ref() else {
                                continue;
                            };

                            let limits = EnforcedLimits::from_events(&program_content, &events);

                            job_stack.clear();
                            job_stack.extend(limits.schedule.iter().rev().copied());

                            enforced_limits = Some(limits);
                        }
                    }
                }
                opt_entry = wait_for_next_start(&job_stack) => {
                    match opt_entry {
                        Some(entry) => {
                            let Some(ref enforced_limits) = enforced_limits else {
                                unreachable!();
                            };

                            let mut enforced_limits = enforced_limits.clone();
                            enforced_limits.limits_root_side = entry.limits_to_root;
                            let _ = enforced_limits;
                        }
                        None => {

                        }
                    }
                }
            }
        }
    });

    Ok(())
}

// https://github.com/tdittr/everest-core/blob/openadr/types/energy.yaml#L213
#[derive(Debug, Clone)]
struct EnforcedLimits {
    uuid: String,
    valid_until: chrono::DateTime<chrono::Utc>,
    limits_root_side: LimitsRes,
    schedule: Vec<ScheduleResEntry>,
}

impl EnforcedLimits {
    fn from_events(program_content: &ProgramContent, events: &[EventClient]) -> Self {
        let events = events.iter().map(|e| e.data()).collect();
        let timeline = Timeline::from_events(program_content, events).unwrap();
        Self::from_timeline(timeline)
    }

    fn from_timeline(timeline: Timeline) -> Self {
        let mut schedule = Vec::new();
        let mut valid_until = None;

        for (range, interval) in timeline.iter() {
            valid_until = Ord::max(valid_until, Some(range.end.clone()));

            let entry = ScheduleResEntry {
                timestamp: range.start,
                limits_to_root: LimitsRes::try_from_event_values(interval.value_map).unwrap(),
            };

            schedule.push(entry);
        }

        Self {
            uuid: Uuid::new_v4().to_string(),
            valid_until: valid_until.unwrap(),
            limits_root_side: LimitsRes { total_power_w: 0.0 },
            schedule,
        }
    }
}

// https://github.com/tdittr/everest-core/blob/openadr/types/energy.yaml#L125
#[derive(Debug, Clone, Copy)]
struct ScheduleResEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    limits_to_root: LimitsRes,
}

// https://github.com/tdittr/everest-core/blob/openadr/types/energy.yaml#L46
#[derive(Debug, Clone, Copy)]
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
