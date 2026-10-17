use super::*;

fn source(frames: &[[f32; 2]], last: Instant) -> Source {
    Source {
        queue: Arc::new(Queue::new(48_000)),
        staged: frames.iter().copied().collect(),
        last,
    }
}

#[test]
fn queue_keeps_order_and_drops_when_full() {
    let queue = Queue::new(48_000);
    for i in 0..CAPACITY + 1 {
        queue.push([i as f32, -(i as f32)]);
    }
    assert_eq!(queue.pop(), Some([0.0, -0.0]));
    assert_eq!(queue.pop(), Some([1.0, -1.0]));
    let mut count = 2;
    while queue.pop().is_some() {
        count += 1;
    }
    assert_eq!(count, CAPACITY);
}

#[test]
fn live_streams_are_summed_frame_by_frame() {
    let now = Instant::now();
    let mut sources = [
        source(&[[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]], now),
        source(&[[0.1, -0.2], [0.1, -0.4]], now),
    ];
    let mixed = mix(&mut sources, now);
    assert_eq!(mixed.len(), 2);
    assert!((mixed[0][0] - 0.2).abs() < 1e-6 && mixed[0][1].abs() < 1e-6);
    assert!((mixed[1][0] - 0.4).abs() < 1e-6 && mixed[1][1].abs() < 1e-6);
    // The longer stream keeps its third frame for the next sum.
    assert_eq!(sources[0].staged, [[0.5, 0.6]]);
}

#[test]
fn an_idle_stream_does_not_hold_back_the_others() {
    let now = Instant::now();
    let mut sources = [
        source(&[[0.5, 0.5]; 3], now),
        source(&[], now - IDLE - Duration::from_millis(1)),
    ];
    assert_eq!(mix(&mut sources, now), vec![[0.5, 0.5]; 3]);
    assert!(mix(&mut [], now).is_empty());
}
