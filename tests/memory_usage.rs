use rdf_glance::RdfGlanceApp;
use peak_alloc::PeakAlloc;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;


#[test]
fn test_memory_usage() {
    // cargo test --test memory_usage -- --nocapture
    let current_mem = PEAK_ALLOC.current_usage_as_kb();
	println!("This program currently uses {} kB of RAM.", current_mem);
    
    let mut app = RdfGlanceApp::new(None);
    app.load_ttl("sample-rdf-data/programming_languages.ttl", true);
    app.join_load(false);
    assert!(app.rdf_data.read().unwrap().node_data.len()>0);
    let mem_after_loading = PEAK_ALLOC.current_usage_as_kb();
	
	println!(" !! Data strucures use {} kB of RAM.", mem_after_loading-current_mem);
    // IriIndex usize
    //  !! Data strucures use 696.39355 kB of RAM.
    // IriIndex u32
    //  !! Data strucures use 584.77246 kB of RAM.
    // String -> Box<str>
    //  !! Data strucures use 538.6553 kB of RAM.
    // StringInterner
    //  !! Data strucures use 537.81934 kB of RAM.
    // StringCache nad StringIndexer for small literals
    //  !! Data strucures use 480 kB of RAM.
}