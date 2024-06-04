use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TargetMap(Vec<TargetEntry>);

// TODO: Handle strong typing of values
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TargetEntry {
    #[serde(rename = "type")]
    label: TargetLabel,
    values: [String; 1],
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TargetLabel {
    /// A Power Service Location is a utility named specific location in
    /// geography or the distribution system, usually the point of service to a
    /// customer site.
    PowerServiceLocation,
    /// A Service Area is a utility named geographic region.
    ServiceArea,
    /// Targeting a specific group (string).
    Group,
    /// Targeting a specific resource (string).
    ResourceName,
    /// Targeting a specific VEN (string).
    #[serde(rename = "VEN_NAME")]
    VENName,
    /// Targeting a specific event (string).
    EventName,
    /// Targeting a specific program (string).
    ProgramName,
    /// An application specific privately defined target.
    #[serde(untagged)]
    Private(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_serialization() {
        assert_eq!(
            serde_json::to_string(&TargetLabel::EventName).unwrap(),
            r#""EVENT_NAME""#
        );
        assert_eq!(
            serde_json::to_string(&TargetLabel::Private(String::from("something else"))).unwrap(),
            r#""something else""#
        );
        assert_eq!(
            serde_json::from_str::<TargetLabel>(r#""VEN_NAME""#).unwrap(),
            TargetLabel::VENName
        );
        assert_eq!(
            serde_json::from_str::<TargetLabel>(r#""something else""#).unwrap(),
            TargetLabel::Private(String::from("something else"))
        );
    }
}