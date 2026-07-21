//! PoC-2 automerge-rs CRDT 同步测试
#![forbid(unsafe_code)]

const _: () = {
    use std::time::Instant;
    use sparkfox_crdt::MemoryDoc;

    #[test]
    fn poc2_1000_entries_sync_under_2s() -> anyhow::Result<()> {
        let mut doc_a = MemoryDoc::new();
        for i in 0..1000 {
            // 修复 Y-07: 传播 set_entry 错误，避免假阳性
            doc_a.set_entry(&format!("entry_{i}"), format!("value_{i}"))?;
        }
        let mut doc_b = MemoryDoc::new();
        let t = Instant::now();
        let sync_msg = doc_a.generate_sync_message();
        doc_b.receive_sync_message(sync_msg)?;
        let elapsed = t.elapsed();
        assert!(elapsed.as_secs_f64() < 2.0, "1000 条同步 {elapsed:?} 超过 2s");
        assert_eq!(doc_b.entry_count(), 1000);
        Ok(())
    }

    #[test]
    fn poc2_offline_then_sync_no_conflict_loss() -> anyhow::Result<()> {
        let mut doc_a = MemoryDoc::new();
        let mut doc_b = MemoryDoc::new();
        doc_a.set_entry("k", "v1")?;
        // 同步一次
        let msg = doc_a.generate_sync_message();
        doc_b.receive_sync_message(msg)?;
        // 离线后双方都改
        doc_a.set_entry("k", "vA_edit")?;
        doc_b.set_entry("k", "vB_edit")?;
        // 双向同步
        let m1 = doc_a.generate_sync_message();
        doc_b.receive_sync_message(m1)?;
        let m2 = doc_b.generate_sync_message();
        doc_a.receive_sync_message(m2)?;
        // CRDT 保证最终一致（最后写入获胜 LWW）
        assert_eq!(doc_a.get_entry("k"), doc_b.get_entry("k"), "最终必须一致");
        Ok(())
    }

    #[test]
    fn poc2_3way_concurrent_no_data_loss() -> anyhow::Result<()> {
        let mut docs: Vec<MemoryDoc> = (0..3).map(|_| MemoryDoc::new()).collect();
        for i in 0..100 {
            docs[i % 3].set_entry(&format!("k_{i}"), format!("v_{i}"))?;
        }
        // 全互连同步
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    let m = docs[i].generate_sync_message();
                    docs[j].receive_sync_message(m)?;
                }
            }
        }
        // 三方数据应一致
        let n = docs[0].entry_count();
        assert!(n >= 100, "条目数应至少 100，实际 {n}");
        for i in 1..3 {
            assert_eq!(docs[i].entry_count(), n, "peer {i} 条目数不一致");
        }
        Ok(())
    }
};
