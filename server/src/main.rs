use bevy::prelude::*;
use clap::Parser;
use server_plugin::ServerPlugin;

mod build_info;
mod server_plugin;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 5001)]
    port: u16,

    /// Transport type (websocket or webtransport)
    #[arg(short, long, default_value = "websocket")]
    transport: String,
}

fn main() {
    let args = Args::parse();
    let build_info = build_info::BuildInfo::get();
    
    // Display the logo at startup

    println!(
        r#"


    ╔══════════════════════════════════════════════════════════════╗
    ║                                                              ║
    ║     ██╗   ██╗ ██████╗ ██╗██████╗                             ║
    ║     ██║   ██║██╔═══██╗██║██╔══██╗                            ║
    ║     ██║   ██║██║   ██║██║██║  ██║                            ║
    ║     ╚██╗ ██╔╝██║   ██║██║██║  ██║                            ║
    ║      ╚████╔╝ ╚██████╔╝██║██████╔╝                            ║
    ║       ╚═══╝   ╚═════╝ ╚═╝╚═════╝                             ║
    ║                                                              ║
    ║     ██╗      ██████╗  ██████╗ ██████╗                        ║
    ║     ██║     ██╔═══██╗██╔═══██╗██╔══██╗                       ║
    ║     ██║     ██║   ██║██║   ██║██████╔╝                       ║
    ║     ██║     ██║   ██║██║   ██║██╔═══╝                        ║
    ║     ███████╗╚██████╔╝╚██████╔╝██║                            ║
    ║     ╚══════╝ ╚═════╝  ╚═════╝ ╚═╝                            ║
    ║                                                              ║
    ║      ██████╗ ██╗   ██╗███████╗███████╗████████╗              ║
    ║     ██╔═══██╗██║   ██║██╔════╝██╔════╝╚══██╔══╝              ║
    ║     ██║   ██║██║   ██║█████╗  ███████╗   ██║                 ║
    ║     ██║▄▄ ██║██║   ██║██╔══╝  ╚════██║   ██║                 ║
    ║     ╚██████╔╝╚██████╔╝███████╗███████║   ██║                 ║
    ║      ╚══▀▀═╝  ╚═════╝ ╚══════╝╚══════╝   ╚═╝                 ║
    ║                                                              ║
    ║                  🚀 Server Starting... 🚀                    ║
    ╚══════════════════════════════════════════════════════════════╝
    "#
    );
    info!("🎮 Simple Platformer Server starting...");
    info!("📡 Listening on port {}", args.port);
    info!("📋 {}", build_info.format_for_log());
    info!("🔧 Build Details:");
    info!("   Git SHA: {}", build_info.git_sha);
    info!("   Git Branch: {}", build_info.git_branch);
    info!("   Build Time: {}", build_info.build_timestamp);
    info!("   Target: {}", build_info.target_triple);
    info!("   Author: {}", build_info.git_commit_author);
    info!("   System: {}", build_info.system_info);

    App::new().add_plugins(ServerPlugin).run();
}
