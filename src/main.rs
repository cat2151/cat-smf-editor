use cat_smf_editor::host as ffi;
mod app;
mod cursor;
#[path = "../../cat-plugin-player/shared/first_instrument_install.rs"]
mod first_instrument_install;
mod play;
mod player;
mod store;
mod view;

fn main() -> eframe::Result<()> {
    cat_smf_editor::startup_trace::init();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        store::error_log::write("panic", &info.to_string());
        previous(info);
    }));
    let result = run();
    if let Err(error) = &result {
        store::error_log::write("startup", &error.to_string());
    }
    result
}

fn run() -> eframe::Result<()> {
    let (player, mid) = match store::cli::parse(std::env::args_os().skip(1)) {
        Ok(store::cli::Arguments::Run { player, mid }) => (player, mid),
        Ok(store::cli::Arguments::Help) => {
            println!("{}", store::cli::USAGE);
            return Ok(());
        }
        Err(error) => {
            store::error_log::write("arguments", &error);
            eprintln!("{error}\n{}", store::cli::USAGE);
            std::process::exit(2);
        }
    };
    let cache = cat_smf_editor::storage_paths::cache_directory()
        .ok_or_else(|| eframe::Error::AppCreation("Cache directory unavailable".into()))?;
    {
        let _span = cat_smf_editor::startup_trace::span("surge_data_home");
        // SAFETY: no GUI, plugin library, or application worker has started yet.
        if let Err(error) = unsafe { cat_smf_editor::surge_data_home::before_threads(&cache) } {
            store::error_log::write("surge-data-home", &error);
        }
    }
    let _native_library = {
        let _span = cat_smf_editor::startup_trace::span("native_library");
        cat_smf_editor::native_library::load_at(&cache).map_err(eframe::Error::AppCreation)?
    };
    let app = app::App::bootstrap(player, mid);
    let _settings_span = cat_smf_editor::startup_trace::span("window_settings");
    let settings = crate::store::config::directory()
        .ok_or_else(|| "LOCALAPPDATA is unavailable".to_owned())
        .and_then(|directory| store::settings::Settings::open(&directory));
    let settings_error = settings.as_ref().err().cloned();
    let settings = settings.unwrap_or_default();
    drop(_settings_span);
    let options = eframe::NativeOptions {
        viewport: settings.viewport(),
        ..Default::default()
    };
    eframe::run_native(
        "cat-smf-editor",
        options,
        Box::new(move |cc| {
            let app = app.attach(cc, settings_error);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod test_support;
