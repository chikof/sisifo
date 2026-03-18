mod commands;

use gateway::start_gateway;
use node::SisiNode;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().unwrap();

			tracing::info!("data_dir = {:?}", data_dir);

            tauri::async_runtime::spawn(async move {
                SisiNode::start(&data_dir)
                    .await
                    .expect("node failed to start");
                start_gateway(7777).await;
            });

            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                tauri::async_runtime::spawn(SisiNode::shutdown());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::publish_site,
            commands::resolve_address,
            commands::list_local_sites,
            commands::node_stats,
            commands::node_identity,
            commands::daemon_running,
            commands::unpin_site,
            commands::pick_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
