use candle_core::Device;
use candle_core::safetensors::load;
use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: dump_safetensors <path>");
        return Ok(());
    }
    let tensors = load(&args[1], &Device::Cpu)?;
    let mut keys: Vec<_> = tensors.keys().collect();
    keys.sort();
    for key in keys {
        println!("{}", key);
    }
    Ok(())
}
