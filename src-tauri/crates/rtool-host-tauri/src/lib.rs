pub mod app;
mod bootstrap;
mod constants;
mod features;
mod host;
mod platform;
mod shared;

pub fn run(context: tauri::Context<tauri::Wry>) {
    bootstrap::AppBootstrap::run(context);
}
