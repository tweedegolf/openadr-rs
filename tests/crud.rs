use std::fs::File;
use serde::Deserialize;
use serde_with::serde_derive::Serialize;
use openadr::Client;
use openadr::wire::{Event, Program, Report};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TestCase {
    programs: Vec<Program>,
    events: Vec<Event>,
    reports: Vec<Report>,
}

#[test]
fn crud() {
    let f = File::open("tests/state-of-charge.oadr.yaml").unwrap();
    let test_data: TestCase = serde_yaml::from_reader(f).unwrap();
    
    Client::new()
}