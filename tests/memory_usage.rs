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
    app.load_ttl("sample-rdf-data/programming_languages.ttl");
    assert!(app.node_data.len()>0);
    let mem_after_loading = PEAK_ALLOC.current_usage_as_kb();
	
	println!(" !! Data strucures use {} kB of RAM.", mem_after_loading-current_mem);
    // IriIndex usize
    //  !! Data strucures use 696.39355 kB of RAM.
    // IriIndex u32
    //  !! Data strucures use 584.77246 kB of RAM.
    // String -> Box<str>
    //  !! Data strucures use 538.6865 kB of RAM.
}