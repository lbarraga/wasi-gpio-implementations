use clap::Parser;
use wasi_gpio::{WasiGpioCtx, WasiGpioView};
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker},
};
use wasmtime_wasi::{
    ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, p2::add_to_linker_sync,
};

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

// Implement the view trait from your library
impl WasiGpioView for HostState {
    fn gpio_ctx(&mut self) -> &mut WasiGpioCtx {
        &mut self.gpio_ctx
    }

    // Since your trait in ctx.rs requires table() as well, we implement it here.
    // (It forwards to the same field as WasiView::table)
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

fn main() -> anyhow::Result<()> {
    // 1. Parse CLI arguments
    let config = HostConfig::parse();

    // 2. Load policies
    let policies = config.get_policies();
    let component_path = config.get_component_path();

    // 3. Initialize Wasmtime engine
    let mut wasm_config = Config::new();
    wasm_config.wasm_component_model(true);
    let engine = Engine::new(&wasm_config)?;
    let mut linker = Linker::new(&engine);

    // 4. Add WASI standard bindings
    add_to_linker_sync(&mut linker)?;

    // 5. Add your GPIO bindings
    wasi_gpio::add_to_linker(&mut linker)?;

    // 6. Initialize the Store
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_network()
        .build();

    let state = HostState {
        ctx: wasi,
        table: ResourceTable::new(),
        gpio_ctx: WasiGpioCtx::new(policies),
    };

    let mut store = Store::new(&engine, state);

    // 7. Load and instantiate
    let component = Component::from_file(&engine, component_path)?;
    let instance = linker.instantiate(&mut store, &component)?;

    // 8. Run the 'start' export (Note: your guest.wit exports 'start', not 'run')
    // If your guest.wit says `export start: func();`, use "start" here.
    let run = instance.get_typed_func::<(), ()>(&mut store, "start")?;
    run.call(&mut store, ())?;

    Ok(())
}
