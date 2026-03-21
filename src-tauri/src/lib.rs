mod commands;

use gateway::start_gateway;
use gossip::init_gossip;
use node::{NodeConfig, SisiNode};
use std::env::var;
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
                let config = if let (Ok(relay), Ok(pkarr), Ok(dns)) = (
                    var("SISI_RELAY_URL"),
                    var("SISI_PKARR_URL"),
                    var("SISI_DNS_ORIGIN"),
                ) {
                    NodeConfig::custom(&relay, &pkarr, &dns)
                } else {
                    NodeConfig::default()
                };

                SisiNode::start(&data_dir, config)
                    .await
                    .expect("node failed to start");
                init_gossip().await.expect("gossip failed to start");
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
            commands::update_site,
            commands::resolve_address,
            commands::list_local_sites,
            commands::node_stats,
            commands::node_identity,
            commands::daemon_running,
            commands::unpin_site,
            commands::pick_folder,
            commands::post_message,
            commands::reply_message,
            commands::list_messages,
            commands::list_replies,
            commands::subscribe_topic,
            commands::delete_message,
            commands::block_user,
            commands::unblock_user,
            commands::get_blocked_users,
            commands::mod_block_user,
            commands::index_site,
            commands::search_sites,
            commands::recent_sites,
            commands::index_count,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
