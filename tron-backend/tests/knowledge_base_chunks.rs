mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
use fixture::*;
use tron_backend::application::knowledge_base::local_documents::{ChunkConfig, ChunkMode};

#[tokio::test]
async fn published_chunks_are_complete_paged_private_and_not_reparsed() {
    let f = Fixture::new();
    let kb = f.service.create(USER, "Chunks", "").unwrap();
    let body = (0..45)
        .map(|n| format!("# Section {n}\nEvidence {n} {}\n", "content ".repeat(30)))
        .collect::<String>();
    let doc = f
        .service
        .upload(USER, &kb.id, "guide.md", body.as_bytes())
        .unwrap();
    assert_eq!(
        f.service
            .published_chunks(USER, &kb.id, &doc.id, 1, 15)
            .unwrap()
            .total,
        0
    );
    f.service
        .configure(
            USER,
            &kb.id,
            &doc.id,
            Some(ChunkConfig {
                mode: ChunkMode::Fixed,
                max_chars: 200,
                overlap_chars: 0,
                heading_depth: 2,
            }),
        )
        .unwrap();
    let ready = f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(ready.status, "READY", "{:?}", ready.error);
    assert!(ready.chunk_count > 20);
    let mut units = vec![];
    for page in 1..=ready.chunk_count.div_ceil(15) {
        let result = f
            .service
            .published_chunks(USER, &kb.id, &doc.id, page, 15)
            .unwrap();
        assert_eq!(result.total, ready.chunk_count);
        assert_eq!(result.page, page);
        units.extend(result.items);
    }
    assert_eq!(units.len(), ready.chunk_count);
    for (n, unit) in units.iter().enumerate() {
        assert_eq!(unit.ordinal, n as i64);
        assert!(!unit.text.is_empty());
        assert_eq!(unit.metadata["documentId"], doc.id);
        assert!(unit.metadata["location"].is_object());
    }
    assert!(
        f.service
            .published_chunks("OTHER-USER", &kb.id, &doc.id, 1, 15)
            .is_err()
    );
    assert!(
        f.service
            .published_chunks(USER, &kb.id, &doc.id, 1, 15)
            .is_err()
    );
    assert!(
        f.service
            .published_chunks(USER, &kb.id, "missing", 1, 15)
            .is_err()
    );
    assert!(
        f.service
            .published_chunks(USER, &kb.id, &doc.id, 0, 15)
            .is_err()
    );
    assert!(
        f.service
            .published_chunks(USER, &kb.id, &doc.id, 1, 101)
            .is_err()
    );
    assert!(
        f.service
            .published_chunks(USER, &kb.id, &doc.id, 1000, 15)
            .unwrap()
            .items
            .is_empty()
    );
    // Settings can change without replacing the published generation. Browsing stays read-only.
    f.service
        .configure(
            USER,
            &kb.id,
            &doc.id,
            Some(ChunkConfig {
                mode: ChunkMode::Fixed,
                max_chars: 400,
                overlap_chars: 0,
                heading_depth: 2,
            }),
        )
        .unwrap();
    let retained = f
        .service
        .published_chunks(USER, &kb.id, &doc.id, 1, 15)
        .unwrap();
    assert_eq!(retained.total, ready.chunk_count);
    assert_eq!(retained.items[0].text, units[0].text);
    let rebuilt = f.service.index(USER, &kb.id, &doc.id).await.unwrap();
    assert_eq!(rebuilt.status, "READY", "{:?}", rebuilt.error);
    assert!(rebuilt.chunk_count < ready.chunk_count);
    let current = f
        .service
        .published_chunks(USER, &kb.id, &doc.id, 1, 100)
        .unwrap();
    assert_eq!(current.total, rebuilt.chunk_count);
    assert_eq!(current.items.len(), rebuilt.chunk_count);
    assert_ne!(current.items[0].text_unit_code, units[0].text_unit_code);
}
