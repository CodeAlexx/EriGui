//! Wave 3 (C3): tests for `NodeGraph` per-node live-progress state.
//!
//! These cover the setter / clearer / error-mark API exposed by
//! `erigui-widgets::node_graph::progress`. Pure CPU — no GPU, no
//! draw-context exercise, just state transitions.

use erigui_core::WidgetId;
use erigui_widgets::{Graph, NodeGraph};

fn id() -> WidgetId {
    WidgetId::default()
}

fn fresh() -> NodeGraph {
    NodeGraph::new(id(), Graph::default())
}

#[test]
fn progress_event_updates_node_state() {
    // The C1 executor pushes `NodeStep` events; the application's main loop
    // turns each into a `set_node_progress(...)` call. This test models that
    // single hop and verifies the widget records the new value.
    let mut g = fresh();
    g.set_node_progress(7, 10, 30, "step");

    let np = g
        .node_progress
        .get(&7)
        .expect("expected progress entry for node 7 after set_node_progress");
    assert_eq!(np.current, 10, "current step should match what was set");
    assert_eq!(np.total, 30, "total step count should match what was set");
    assert_eq!(np.label, "step", "label should match what was set");
    assert!(np.error.is_none(), "fresh progress should not be errored");

    // The convenience accessor reads the same map.
    let np2 = g
        .node_progress_for(7)
        .expect("node_progress_for should mirror the underlying map");
    assert_eq!(np2.current, 10);

    // Sanity: fraction = current/total.
    assert!((np.fraction() - 10.0 / 30.0).abs() < 1e-6);
}

#[test]
fn progress_clears_on_done() {
    // Spec contract: when the executor emits `NodeDone`, the host calls
    // `clear_node_progress`. After the call the entry is gone entirely.
    let mut g = fresh();
    g.set_node_progress(7, 10, 30, "step");
    assert!(g.node_progress.contains_key(&7), "precondition: entry exists");

    g.clear_node_progress(7);
    assert!(
        !g.node_progress.contains_key(&7),
        "clear_node_progress should remove the entry from the map"
    );
    assert!(g.node_progress.is_empty(), "map should be empty after clear");
}

#[test]
fn error_overrides_running_state() {
    // When the node errors out mid-run, the bar should *stay* on screen but
    // flip to the error variant. The entry must still exist; only the
    // `error` field is populated.
    let mut g = fresh();
    g.set_node_progress(7, 10, 30, "klein euler");
    g.set_node_error(7, "out of memory");

    let np = g
        .node_progress
        .get(&7)
        .expect("entry must persist after set_node_error so the red bar can render");
    assert_eq!(
        np.error.as_deref(),
        Some("out of memory"),
        "error message should be stored verbatim"
    );
    assert!(np.is_error(), "is_error() must reflect the error state");
    // Pre-existing step/label state is preserved (the bar can still show
    // "10/30" with a red fill).
    assert_eq!(np.current, 10);
    assert_eq!(np.total, 30);
    assert_eq!(np.label, "klein euler");

    // A fresh `set_node_progress` after an error means a new run started;
    // the error should clear.
    g.set_node_progress(7, 1, 30, "klein euler");
    let np = g.node_progress.get(&7).unwrap();
    assert!(
        np.error.is_none(),
        "starting a new run should clear the prior error"
    );
}

#[test]
fn set_node_error_without_prior_progress_creates_entry() {
    // Defensive: if `set_node_error` arrives before any `set_node_progress`
    // call (e.g. a node that fails at validation), we still create the
    // entry so the indicator paints.
    let mut g = fresh();
    g.set_node_error(42, "missing input");
    let np = g.node_progress.get(&42).expect("error-first should create entry");
    assert!(np.is_error());
    assert_eq!(np.current, 0);
    assert_eq!(np.total, 0);
}

#[test]
fn fraction_handles_zero_total() {
    // total == 0 means "indeterminate" — the renderer treats it as 0.0
    // fill so only the label shows; verify the helper doesn't divide by
    // zero.
    let mut g = fresh();
    g.set_node_progress(1, 0, 0, "starting");
    let np = g.node_progress.get(&1).unwrap();
    assert_eq!(np.fraction(), 0.0);
}
