use crate::platform::{Platform, WslStatus};
use anyhow::Result as AnyResult;
pub fn run() -> AnyResult<()> {
    let platform: Platform = Platform::detect();
    println!("dsi info");
    println!();
    println!("  dsi version:    {}", env!("CARGO_PKG_VERSION"));
    println!("  Platform:       {}", platform.display_name());
    println!("  Architecture:   {}", std::env::consts::ARCH);
    println!("  OS:             {:?}", platform.os);

    match platform.wsl {
        WslStatus::None => println!("  WSL:            NO"),
        WslStatus::Wsl1 => println!("  WSL:            WSL1"),
        WslStatus::Wsl2 => println!("  WSL:            WSL2"),
    }
    println!();
    Ok(())
}
