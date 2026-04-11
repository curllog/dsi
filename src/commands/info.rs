pub fn run(){
    println!("dsi version: {}", env!("CARGO_PKG_VERSION"));
    println!("Platform: {}", std::env::consts::OS);
    println!("Architecture: {}", std::env::consts::ARCH);
}