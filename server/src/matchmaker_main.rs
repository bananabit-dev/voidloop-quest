mod matchmaker;

#[cfg(feature = "matchmaker")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 Voidloop Quest - Matchmaker Service");
    println!("🔐 Securely handling Edgegap API with server-side token");
    
    matchmaker::run_matchmaker_service().await
}

#[cfg(not(feature = "matchmaker"))]
fn main() {
    eprintln!("❌ Matchmaker service requires 'matchmaker' feature");
    eprintln!("Run with: cargo run --bin matchmaker --features matchmaker");
    std::process::exit(1);
}