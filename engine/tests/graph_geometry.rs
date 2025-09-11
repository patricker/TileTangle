use engine as eng;
use engine::BoardGeometry;
use engine::Rules;

fn make_cfg(w: u32, h: u32) -> eng::GameConfig {
    let tileset = eng::Tileset { tile_kinds: vec![
        eng::TileKind { id:"A".into(), symbol:"A".into(), score:1, is_blank:false, aliases:vec![] },
        eng::TileKind { id:"B".into(), symbol:"B".into(), score:3, is_blank:false, aliases:vec![] },
    ]};
    let mut counts = std::collections::HashMap::new(); counts.insert("A".into(), 50); counts.insert("B".into(), 20);
    eng::GameConfig { tileset, rack_size:7, board_layout: eng::RectBoardLayout { width: w, height: h }, ruleset_id:"cross".into(), dictionary_id:"en".into(), rng_seed:1, tile_counts: counts }
}

#[test]
fn hex_neighbors_counts() {
    let cfg = make_cfg(7, 7);
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    // Build even-r hex overlay
    let mut nodes = Vec::new();
    for y in 0..7i32 { for x in 0..7i32 { nodes.push(eng::Coord2D { x, y }); } }
    let idx = |x:i32,y:i32| (y*7 + x) as usize;
    let mut edges: Vec<(usize,usize,String)> = Vec::new();
    let try_edge = |edges: &mut Vec<(usize,usize,String)>, x1:i32,y1:i32,x2:i32,y2:i32, dir:&str| {
        if x2<0||x2>=7||y2<0||y2>=7 { return; }
        edges.push((idx(x1,y1), idx(x2,y2), dir.to_string()));
    };
    for y in 0..7i32 { for x in 0..7i32 {
        let even = (y%2)==0;
        try_edge(&mut edges, x,y, x+1,y, "E");
        try_edge(&mut edges, x,y, x + if even{0}else{1}, y-1, "NE");
        try_edge(&mut edges, x,y, x + if even{0}else{1}, y+1, "SE");
    }}
    st.apply_graph_overlay(eng::GraphOverlay { nodes, edges }).unwrap();
    // center should have 6 neighbors
    let c = st.board.geom.to_cell_id(eng::Coord2D { x:3, y:3 }).unwrap();
    assert_eq!(st.board.geom.neighbors(c).len(), 6);
}

#[test]
fn graph_line_and_contiguity() {
    let cfg = make_cfg(5, 5);
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    // Build simple straight line along row y=2 with tag "E"
    let nodes = (0..5).map(|x| eng::Coord2D { x, y:2 }).collect::<Vec<_>>();
    let mut edges = Vec::new();
    for x in 0..4usize { edges.push((x, x+1, "E".into())); }
    st.apply_graph_overlay(eng::GraphOverlay { nodes, edges }).unwrap();
    let rules = eng::CrosswordRules { free_word_mode: true, ..Default::default() };
    // Place at (1,2) and (3,2) with an existing tile at (2,2) bridging the gap
    let id2 = st.board.geom.to_cell_id(eng::Coord2D { x:2, y:2 }).unwrap();
    st.board.cells[id2.0 as usize].stack.push(eng::Tile { kind_id:"A".into(), mark: None });
    let mv = eng::MoveDraft { placements: vec![
        (st.board.geom.to_cell_id(eng::Coord2D { x:1, y:2 }).unwrap(), eng::Tile { kind_id:"B".into(), mark: None }),
        (st.board.geom.to_cell_id(eng::Coord2D { x:3, y:2 }).unwrap(), eng::Tile { kind_id:"B".into(), mark: None }),
    ]};
    let v = rules.validate(&st, &mv).unwrap();
    let sc = rules.score(&st, &v);
    assert!(sc.total >= 0);
}
