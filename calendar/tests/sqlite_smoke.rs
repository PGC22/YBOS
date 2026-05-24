#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use ybos_calendar::{Event, EventType, EventQuery, SqliteCalendarStore, CalendarStore};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_sqlite_calendar_smoke() {
        let store = SqliteCalendarStore::in_memory().unwrap();

        let id1 = Uuid::new_v4();
        let event1 = Event {
            id: id1,
            title: "Internal Meeting".to_string(),
            description: Some("Discussion about Y10".to_string()),
            start_ts: 1000,
            end_ts: 2000,
            location: Some("Office".to_string()),
            event_type: EventType::Internal,
            attendees: vec!["Alice".to_string(), "Bob".to_string()],
            created_at: 100,
            updated_at: 100,
        };

        let id2 = Uuid::new_v4();
        let event2 = Event {
            id: id2,
            title: "External Call".to_string(),
            description: None,
            start_ts: 3000,
            end_ts: 4000,
            location: None,
            event_type: EventType::External,
            attendees: vec!["Charlie".to_string()],
            created_at: 200,
            updated_at: 200,
        };

        let id3 = Uuid::new_v4();
        let event3 = Event {
            id: id3,
            title: "Personal Errands".to_string(),
            description: None,
            start_ts: 500,
            end_ts: 600,
            location: None,
            event_type: EventType::Personal,
            attendees: vec![],
            created_at: 300,
            updated_at: 300,
        };

        store.put(event1.clone()).await.unwrap();
        store.put(event2.clone()).await.unwrap();
        store.put(event3.clone()).await.unwrap();

        assert_eq!(store.count().await.unwrap(), 3);

        // Get by id
        let fetched = store.get(id1).await.unwrap().expect("Event should exist");
        assert_eq!(fetched.title, "Internal Meeting");

        // Update
        let mut updated_event1 = event1.clone();
        updated_event1.end_ts = 2500;
        updated_event1.updated_at = 150;
        store.put(updated_event1).await.unwrap();

        let fetched_updated = store.get(id1).await.unwrap().expect("Event should exist");
        assert_eq!(fetched_updated.end_ts, 2500);
        assert_eq!(fetched_updated.created_at, 100);
        assert_eq!(fetched_updated.updated_at, 150);

        // List all (ordered by start_ts)
        let all = store.list(EventQuery::default()).await.unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, id3); // 500
        assert_eq!(all[1].id, id1); // 1000
        assert_eq!(all[2].id, id2); // 3000

        // List with filter
        let filtered = store.list(EventQuery {
            from_ts: Some(800),
            to_ts: Some(3500),
            event_type: None,
            limit: 0,
        }).await.unwrap();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, id1);
        assert_eq!(filtered[1].id, id2);

        let type_filtered = store.list(EventQuery {
            event_type: Some(EventType::External),
            ..Default::default()
        }).await.unwrap();
        assert_eq!(type_filtered.len(), 1);
        assert_eq!(type_filtered[0].id, id2);

        // Delete
        assert!(store.delete(id3).await.unwrap());
        assert_eq!(store.count().await.unwrap(), 2);
        assert!(store.get(id3).await.unwrap().is_none());
    }
}
