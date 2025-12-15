use clap::Parser;
use wasi_gpio::{WasiGpioCtx, WasiGpioView};
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker},
};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

// We reuse the Config struct from your library for argument parsing,
// or you could define a new one here if you want different CLI args.
use wasi_gpio::policies::Config as HostConfig;

struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    gpio_ctx: WasiGpioCtx,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

// Implement the view trait from your library to give it access to the GPIO context
impl WasiGpioView for HostState {
    fn gpio_ctx(&mut self) -> &mut WasiGpioCtx {
        &mut self.gpio_ctx
    }

    // The library needs access to the resource table as well
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

fn main() -> anyhow::Result<()> {
    // 1. Parse CLI arguments (policy file path, component path)
    let config = HostConfig::parse();

    // 2. Load policies from the specified file
    let policies = config.get_policies();
    let component_path = config.get_component_path();

    // 3. Initialize Wasmtime engine
    let mut wasm_config = Config::new();
    wasm_config.wasm_component_model(true);
    let engine = Engine::new(&wasm_config)?;
    let mut linker = Linker::new(&engine);

    // 4. Add WASI standard bindings to the linker
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;

    // 5. Add your GPIO bindings to the linker
    wasi_gpio::add_to_linker(&mut linker)?;

    // 6. Initialize the Store with our HostState
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_network()
        .build();

    let state = HostState {
        ctx: wasi,
        table: ResourceTable::new(),
        // Initialize the GPIO context with the loaded policies
        gpio_ctx: WasiGpioCtx::new(policies),
    };

    let mut store = Store::new(&engine, state);

    // 7. Load and instantiate the component
    let component = Component::from_file(&engine, component_path)?;
    let instance = linker.instantiate(&mut store, &component)?;

    // 8. Run the 'run' export (assuming a command component)
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    run.call(&mut store, ())?;

    Ok(())
}
