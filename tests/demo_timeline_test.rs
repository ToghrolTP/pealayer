use pealayer::four_d::demo_timeline::{build_demo_showcase_timeline, demo_media_target};

#[test]
fn test_demo_showcase_timeline_structure() {
    let timeline = build_demo_showcase_timeline();
    assert!(!timeline.instances.is_empty(), "timeline must contain effect instances");
    assert!(!timeline.templates.is_empty(), "timeline must contain template definitions");
    assert!(!timeline.analog_tracks.is_empty(), "timeline must contain analog fan curves");

    // Check that seat motion cues exist in templates and are referenced in instances
    let seat_template_ids: Vec<_> = timeline
        .templates
        .iter()
        .filter(|t| t.name.to_lowercase().contains("seat"))
        .map(|t| t.id)
        .collect();
    assert!(!seat_template_ids.is_empty(), "demo timeline must define cinema seat motion template");

    let has_seat_instance = timeline
        .instances
        .iter()
        .any(|inst| seat_template_ids.contains(&inst.effect_id));
    assert!(has_seat_instance, "demo timeline must place cinema seat motion instance");

    let media = demo_media_target();
    assert!(media.is_some(), "demo media target must be provided");
}
