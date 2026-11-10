use pealayer::four_d::demo_timeline::{build_demo_showcase_timeline, demo_media_target};

#[test]
fn test_demo_showcase_timeline_structure() {
    let timeline = build_demo_showcase_timeline();
    assert!(!timeline.instances.is_empty(), "timeline must contain effect instances");
    assert!(!timeline.templates.is_empty(), "timeline must contain template definitions");
    assert!(!timeline.analog_tracks.is_empty(), "timeline must contain analog fan curves");

    for (key, side) in [("seat.a", 0), ("seat.b", 1)] {
        let template = timeline.templates.iter().find(|template| {
            template.direct_control.as_ref().is_some_and(|control| {
                control.control_key == key && control.motion.is_some_and(|motion| motion.side == side)
            })
        }).expect("showcase needs both seat axes");
        assert!(timeline.instances.iter().any(|cue| cue.effect_id == template.id));
    }
    let wind = timeline.templates.iter().find(|template| {
        template.direct_control.as_ref().is_some_and(|control| control.control_key == "relay.5")
    }).expect("wind cue must use an advertised user relay in the cinema profile");
    assert!(wind.actions.iter().all(|action| action.relay_id == 5));

    let media = demo_media_target();
    assert!(media.is_some(), "demo media target must be provided");
}
