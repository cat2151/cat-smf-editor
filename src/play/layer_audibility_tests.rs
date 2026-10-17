use super::*;
struct Constant;
impl Instrument for Constant {
    fn render(&mut self, _: &[u8; 128], _: &[u64; 128], _: i32, out: &mut [f32], _: usize) -> bool {
        out.fill(1.0);
        true
    }
}
struct NoEffect;
impl Effect for NoEffect {
    fn apply(&mut self, _: &mut [f32], _: usize) -> bool {
        true
    }
}
#[test]
fn solos_add_and_mute_always_wins_without_changing_layer_gains() {
    let mut render = Render::<Constant, NoEffect>::new(
        (1..=3)
            .map(|id| {
                (
                    Constant,
                    Layer {
                        id,
                        ..Default::default()
                    },
                )
            })
            .collect(),
        None,
    );
    let mut check = |mute: [bool; 3], solo: [bool; 3], expected: f32| {
        for (i, (_, layer)) in render.layers.iter_mut().enumerate() {
            layer.mute = mute[i];
            layer.solo = solo[i];
        }
        let mut out = [0.0; 2];
        assert!(render.process(&[80; 128], &[200; 128], 1, &mut out, 2));
        assert_eq!(out, [expected; 2]);
    };
    check([false; 3], [false; 3], 3.0);
    check([false; 3], [true, false, false], 1.0);
    check([false; 3], [true, true, false], 2.0);
    check([true, false, false], [true, true, false], 1.0);
    check([true, true, false], [true, true, false], 0.0);
    check([true, true, false], [false; 3], 1.0);
    check([false; 3], [false; 3], 3.0);
}
