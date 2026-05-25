fn main() {
    let path = std::path::Path::new("data/world_core");
    let (idx, stats) = adam_algebra::corpus_loader::load_world_core(path).expect("load");
    println!("Files loaded: {}", stats.files_loaded);
    println!("Entries read: {}", stats.entries_read);
    println!("Frames inserted: {}", stats.frames_inserted);
    println!(
        "Unknown predicate skipped: {}",
        stats.unknown_predicate_skipped
    );
    println!("FrameIndex size: {}", idx.len());
}
