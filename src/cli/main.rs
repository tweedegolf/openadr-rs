#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = openadr::Client::new("http://localhost/openadr/".try_into()?);
    let _program = client.get_program_by_name("name").await?;
    // let events = program.get_all_events().await?;

    // let programs: Vec<Program> = client.get_all_programs()?;
    // let programs = client.get_programs(TargetLabel::ProgramName, &["name"])?;

    // let program = client.get_program_by_id("id").await?;

    // let evt = program.send_event(Event {

    // })?;

    // let events = program.get_events(TargetLabel::EventName, &["name"], 0, 10)?;

    // program.get_event_by_name("prices").await?;

    // let events = program.get_all_events().await?;

    // // find the event you want, each event contains all relevant information to reconstruct periods
    // let event = events[0];

    // for interval in event.intervals {
    //     for payload in interval.payloads { // Iterator<Item = Payload
    //         // do something with the payload
    //         payload.timestamp;
    //         payload.unit;
    //         payload.currency;
    //         payload.value::<T = f64>();
    //     }
    // }


    // send a report
    // event.send_report(Report {

    // }).await?;


    // program.on_event(|evt| {

    // })?;


    Ok(())
}


