use kinema_domain::rope::ParticleRope;

#[test]
fn test_rope_creation_and_defaults() {
    let p0 = [0.0, 2.0];
    let p1 = [2.0, 2.0];
    let rope = ParticleRope::new_catenary(p0, p1, 2.4, 1.2).expect("valid catenary");

    assert_eq!(rope.nodes.len(), 24);
    assert_eq!(rope.relaxation_passes, 12);
    assert!((rope.total_length - 2.4).abs() < 1e-9);
    assert!((rope.total_mass - 1.2).abs() < 1e-9);
    assert_eq!(rope.nodes[0].pos, p0);
    assert!(rope.nodes[0].pinned);
    assert!(!rope.nodes[23].pinned);

    let d0 = rope.rest_segment_length();
    assert!((d0 - (2.4 / 23.0)).abs() < 1e-9);
}

#[test]
fn test_rope_stretch_under_one_percent() {
    // Both ends pinned forming a catenary hanging under Earth gravity
    let mut rope =
        ParticleRope::new_catenary([0.0, 3.0], [2.0, 3.0], 2.5, 1.0).expect("valid rope");
    rope.set_pinned(23, true).expect("pin right end");

    let dt = 1.0 / 240.0;
    // Advance 5 seconds (1200 steps)
    for _ in 0..1200 {
        rope.step(dt);
    }

    let stretch = rope.stretch_ratio();
    // Section 6.5 acceptance criterion: stretch stays strictly under 1% (0.01)
    assert!(
        stretch < 0.01,
        "Rope stretch must stay under 1%, got {:.4}%",
        stretch * 100.0
    );
}

#[test]
fn test_rope_ten_minute_stability_and_no_nan() {
    // 10 minutes of simulated time at 240 Hz = 144,000 steps.
    // In Rust, 144,000 steps of 24 nodes with 12 relaxation passes takes ~30-50ms in dev mode.
    let mut rope =
        ParticleRope::new_catenary([0.0, 5.0], [3.0, 5.0], 4.0, 2.0).expect("valid rope");
    rope.set_pinned(23, true).expect("pin right end");

    let dt = 1.0 / 240.0;
    let total_steps = 600 * 240; // 144,000 steps = 10 minutes

    for step in 0..total_steps {
        rope.step(dt);

        // Periodic invariant check every 10,000 steps to catch any drift or blow-up early
        if step % 10_000 == 0 {
            for (idx, node) in rope.nodes.iter().enumerate() {
                assert!(
                    node.pos[0].is_finite() && node.pos[1].is_finite(),
                    "Node {} became non-finite at step {}: {:?}",
                    idx,
                    step,
                    node.pos
                );
            }
        }
        if step >= 240 && step % 10_000 == 0 {
            assert!(
                rope.stretch_ratio() < 0.01,
                "Stretch exceeded 1% at step {}: {:.4}%",
                step,
                rope.stretch_ratio() * 100.0
            );
        }
    }

    // Final checks after 10 full minutes
    for node in &rope.nodes {
        assert!(node.pos[0].is_finite());
        assert!(node.pos[1].is_finite());
    }
    assert!(rope.stretch_ratio() < 0.01);
}

#[test]
fn test_rope_fuzz_forces_and_no_nan() {
    let mut rope =
        ParticleRope::new_catenary([0.0, 4.0], [2.0, 4.0], 3.0, 1.5).expect("valid rope");

    // Apply erratic pulling forces on end node
    let dt = 1.0 / 240.0;
    for i in 0..2400 {
        let fx = ((i as f64) * 0.13).sin() * 50.0;
        let fy = ((i as f64) * 0.27).cos() * 30.0;
        rope.set_end_pull_force(fx, fy);
        rope.step(dt);
    }

    for (idx, node) in rope.nodes.iter().enumerate() {
        assert!(
            node.pos[0].is_finite() && node.pos[1].is_finite(),
            "Fuzzed pull created NaN/Inf at node {}: {:?}",
            idx,
            node.pos
        );
    }
    assert!(rope.stretch_ratio() < 0.05); // under dynamic fuzzed pulling, remains intact
}

#[test]
fn test_rope_surface_contact_and_friction() {
    let mut rope = ParticleRope::new_catenary([0.0, 1.0], [2.0, 1.0], 2.5, 1.0)
        .expect("valid")
        .with_surface(0.0, 0.4);

    let dt = 1.0 / 240.0;
    for _ in 0..1200 {
        rope.step(dt);
    }

    // No node should fall below surface_y = 0.0
    for node in &rope.nodes {
        assert!(
            node.pos[1] >= -1e-9,
            "Node penetrated ground: y = {}",
            node.pos[1]
        );
    }
}

#[test]
fn test_rope_segment_tensions() {
    let rope = ParticleRope::new_catenary([0.0, 2.0], [2.0, 2.0], 2.5, 1.0).expect("valid");
    let tensions = rope.segment_tensions(1.0 / 240.0);
    assert_eq!(tensions.len(), 23);
    for t in tensions {
        assert!(t.is_finite());
        assert!(t >= 0.0);
    }
}
