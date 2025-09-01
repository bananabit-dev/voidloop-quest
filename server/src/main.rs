use bevy::prelude::*;
use clap::Parser;
use lightyear::prelude::server::TransportConfig;
use server_plugin::ServerPlugin;

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
    
    info!("🎮 Simple Platformer Server starting...");
    info!("📡 Listening on port {}", args.port);
    
    let transport = match args.transport.as_str() {
        "webtransport" => TransportConfig::WebTransportServer {
            server_addr: ([0, 0, 0, 0], args.port).into(),
            certificate: Default::default(),
        },
        _ => TransportConfig::WebSocketServer {
            server_addr: ([0, 0, 0, 0], args.port).into(),
            certificate: Default::default(),
        },
    };
    
    App::new()
        .add_plugins(ServerPlugin { transport })
        .run();
}
