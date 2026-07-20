/// User-owned producers are a closed architectural set. Keep this guard close
/// to the two realtime audience traits so a private domain cannot silently
/// regress to the instance-wide sink during later refactors.
#[test]
fn user_owned_producers_cannot_depend_on_instance_event_broadcaster() {
    let producers = [
        ("channel/manager", include_str!("../../sparkfox-be-channel/src/manager.rs")),
        ("channel/pairing", include_str!("../../sparkfox-be-channel/src/pairing.rs")),
        ("companion/events", include_str!("../../sparkfox-be-companion/src/events.rs")),
        ("conversation/service", include_str!("../../sparkfox-be-conversation/src/service.rs")),
        ("conversation/stream_relay", include_str!("../../sparkfox-be-conversation/src/stream_relay.rs")),
        ("cron/artifacts", include_str!("../../sparkfox-be-cron/src/artifacts.rs")),
        ("cron/events", include_str!("../../sparkfox-be-cron/src/events.rs")),
        ("cron/executor", include_str!("../../sparkfox-be-cron/src/executor.rs")),
        ("cron/skill_suggest", include_str!("../../sparkfox-be-cron/src/skill_suggest.rs")),
        ("file/service", include_str!("../../sparkfox-be-file/src/service.rs")),
        ("file/watch_service", include_str!("../../sparkfox-be-file/src/watch_service.rs")),
        ("idmm/events", include_str!("../../sparkfox-be-idmm/src/events.rs")),
        ("office/watch_manager", include_str!("../../sparkfox-be-office/src/watch_manager.rs")),
        ("terminal/events", include_str!("../../sparkfox-be-terminal/src/events.rs")),
    ];

    for (name, source) in producers {
        assert!(
            source.contains("UserEventSink"),
            "{name} must retain the owner-scoped realtime boundary"
        );
        assert!(
            !source.contains("EventBroadcaster"),
            "{name} must not depend on the instance-wide event boundary"
        );
        assert!(
            !source.contains("WebSocketManager"),
            "{name} must not bypass the shared owner-scoped event bus"
        );
    }
}
