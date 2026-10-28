//! Change the installed-plugin environment only inside a dedicated test process.
#[test]
#[cfg(windows)]
fn editing_and_undo_work_without_the_default_instrument() {
    const CHILD: &str = "CAT_SMF_VARIATION_WITHOUT_PLUGINS";
    if std::env::var_os(CHILD).is_some() {
        // Windows permits set_var even with other threads. The parent test suite's
        // environment is untouched, and this child runs only this exact test.
        unsafe { std::env::set_var("CommonProgramFiles", "X:/ci-without-installed-plugins") };
        assert!(cat_smf_editor::chain::default_instrument().is_err());
        super::keyboard_selection_cancel_confirm_control_undo_and_close();
        super::opening_repeats_the_entire_owner_and_keeps_repeat_after_point_edit();
        super::view_entry_keeps_playback_and_saved_curve_unchanged();
        super::super::pattern_tests::pattern_parameter_curves_keep_local_ticks_baselines_and_other_bodies();
        println!("Completed plugin-free variation checks");
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "view::parameter_editor::variation_tests::plugin_independence::editing_and_undo_work_without_the_default_instrument",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success()
            && String::from_utf8_lossy(&output.stdout)
                .contains("Completed plugin-free variation checks"),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
