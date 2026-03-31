mod render;
mod stats;

use std::path::PathBuf;

use futures::executor::block_on;
use ir::{ExportKind, Instruction, Module};
use mpz_common::context::test_st_context;
use mpz_vm_core_new::{
    Param, VmError,
    ideal::{Instance, TraceEvent},
};

use crate::stats::Stats;

const WASM_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/wasm/profile_bench_programs.wasm"
);

struct Benchmark {
    name: String,
    func_idx: u32,
    /// (ptr, data) pairs describing private memory regions.
    private_regions: Vec<(u32, Vec<u8>)>,
    /// Parameters for the private party (has the data).
    params_a: Vec<Param>,
    /// Parameters for the blind party (receives data via flush).
    params_b: Vec<Param>,
}

fn main() {
    let filter: Option<String> = std::env::args().nth(1);

    let wasm_path = PathBuf::from(WASM_PATH);
    if !wasm_path.exists() {
        eprintln!(
            "WASM binary not found at {}\nRun ./crates/profile-bench/build-wasm.sh first.",
            wasm_path.display()
        );
        std::process::exit(1);
    }

    let wasm_bytes = std::fs::read(&wasm_path).expect("wasm file should be readable");
    let module = Module::parse(&wasm_bytes).expect("wasm should parse");

    let heap_base = find_heap_base(&module);
    let mut benchmarks = discover_benchmarks(&module, heap_base);

    if let Some(ref f) = filter {
        benchmarks.retain(|b| b.name.contains(f.as_str()));
    }

    if benchmarks.is_empty() {
        eprintln!("No benchmark functions found in the WASM binary.");
        std::process::exit(1);
    }

    // Header
    println!(
        "{:<24} | {:>12} | {:>12} | {:>10} | {:>9} | {:>9} | {:>7} | {:>7} | {}",
        "Benchmark",
        "Public CF",
        "Private CF",
        "Priv CF Rgn",
        "Mem Ld",
        "Mem St",
        "Calls",
        "Decode",
        "Result"
    );
    println!("{}", "-".repeat(120));

    let results: Vec<_> = benchmarks
        .iter()
        .map(|bench| {
            let result = run_benchmark(&module, bench);
            println!(
                "{:<24} | {:>12} | {:>12} | {:>10} | {:>9} | {:>9} | {:>7} | {:>7} | {}",
                bench.name,
                result.stats.public_cf_ops,
                result.stats.private_cf_ops,
                result.stats.private_cf_count,
                result.stats.memory_loads,
                result.stats.memory_stores,
                result.stats.call_count,
                result.stats.decode_count,
                result.outcome,
            );
            (bench, result)
        })
        .collect();

    // Print detailed op histograms
    println!();
    for (bench, result) in &results {
        println!("--- {} op histogram (public cf) ---", bench.name);
        let mut ops: Vec<_> = result.stats.public_cf_histogram.iter().collect();
        ops.sort_by(|a, b| b.1.cmp(a.1));
        for (op, count) in &ops {
            println!("  {:<32} {:>6}", op, count);
        }

        println!("--- {} op histogram (private cf) ---", bench.name);
        let mut ops: Vec<_> = result.stats.private_cf_histogram.iter().collect();
        ops.sort_by(|a, b| b.1.cmp(a.1));
        for (op, count) in &ops {
            println!("  {:<32} {:>6}", op, count);
        }
        println!();
    }

    // Generate JSON data files
    let output_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/output"));
    std::fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    for (bench, result) in &results {
        let json = render::render_json(&bench.name, &result.stats, &result.regions, &result.blocks, &result.calls);
        let path = output_dir.join(format!("{}.json", bench.name));
        std::fs::write(&path, json).expect("json file should be writable");
        println!("Wrote {}", path.display());
    }
}

fn find_heap_base(module: &Module) -> u32 {
    for export in module.exports() {
        if export.name == "__heap_base" {
            if let ExportKind::Global(idx) = export.kind {
                let global = &module.globals()[idx as usize];
                if let Some(Instruction::I32Const { val, .. }) = global.init.first() {
                    return *val as u32;
                }
            }
        }
    }
    eprintln!("Warning: __heap_base not found, using 65536 as fallback");
    65536
}

fn discover_benchmarks(module: &Module, heap_base: u32) -> Vec<Benchmark> {
    let mut benchmarks = Vec::new();

    for export in module.exports() {
        let func_idx = match export.kind {
            ExportKind::Func(idx) => idx,
            _ => continue,
        };

        if export.name.starts_with('_') || export.name == "memory" {
            continue;
        }

        let bench = match export.name.as_str() {
            "sha256" => {
                let msg_data = vec![0xABu8; 64];
                let msg_ptr = heap_base;
                let msg_len = msg_data.len() as i32;
                let out_ptr = heap_base + msg_data.len() as u32;
                Benchmark {
                    name: export.name.clone(),
                    func_idx,
                    private_regions: vec![(msg_ptr, msg_data)],
                    params_a: vec![
                        Param::public_i32(msg_ptr as i32),
                        Param::public_i32(msg_len),
                        Param::public_i32(out_ptr as i32),
                    ],
                    params_b: vec![
                        Param::public_i32(msg_ptr as i32),
                        Param::public_i32(msg_len),
                        Param::public_i32(out_ptr as i32),
                    ],
                }
            }
            "json_parse" => {
                static JSON_FIXTURE: &[u8] =
                    include_bytes!("../../profile-bench-programs/fixtures/sample.json");
                let json_data = JSON_FIXTURE.to_vec();
                let json_ptr = heap_base;
                let json_len = json_data.len() as i32;
                Benchmark {
                    name: export.name.clone(),
                    func_idx,
                    private_regions: vec![(json_ptr, json_data)],
                    params_a: vec![
                        Param::public_i32(json_ptr as i32),
                        Param::public_i32(json_len),
                    ],
                    params_b: vec![
                        Param::public_i32(json_ptr as i32),
                        Param::public_i32(json_len),
                    ],
                }
            }
            other => {
                eprintln!("Warning: unknown export '{}', skipping", other);
                continue;
            }
        };

        benchmarks.push(bench);
    }

    benchmarks
}

struct BenchResult {
    stats: Stats,
    regions: Vec<stats::CfRegion>,
    blocks: Vec<stats::BlockInfo>,
    calls: Vec<stats::CallInfo>,
    outcome: String,
}

fn run_benchmark(module: &Module, bench: &Benchmark) -> BenchResult {
    let mut instance_a = match Instance::with_tracing(module.clone()) {
        Ok(i) => i,
        Err(e) => {
            return BenchResult {
                stats: Stats::empty(),
                regions: vec![],
                blocks: vec![],
                calls: vec![],
                outcome: format!("setup error: {}", e),
            };
        }
    };
    let mut instance_b = match Instance::new(module.clone()) {
        Ok(i) => i,
        Err(e) => {
            return BenchResult {
                stats: Stats::empty(),
                regions: vec![],
                blocks: vec![],
                calls: vec![],
                outcome: format!("setup error: {}", e),
            };
        }
    };

    for (ptr, data) in &bench.private_regions {
        let _ = instance_a.mark_private(*ptr, data.len());
        let _ = instance_a.write_private(*ptr, data);
        let _ = instance_b.mark_blind(*ptr, data.len());
    }

    let params_a = bench.params_a.clone();
    let params_b = bench.params_b.clone();
    let (mut ctx_a, mut ctx_b) = test_st_context(8192);

    let outcome = match block_on(futures::future::try_join(
        instance_a.call(&mut ctx_a, bench.func_idx, params_a),
        instance_b.call(&mut ctx_b, bench.func_idx, params_b),
    )) {
        Ok((result_a, _result_b)) => match result_a {
            Some(val) => format!("ok (returned {:?})", val),
            None => "ok".to_string(),
        },
        Err(VmError::Trap(trap)) => format!("TRAP: {}", trap),
        Err(e) => format!("ERROR: {}", e),
    };

    let trace = instance_a
        .trace_log()
        .map(|t| t.to_vec())
        .unwrap_or_default();

    let (stats, regions) = stats::collect(&trace);
    let blocks = stats::collect_blocks(module, &trace);
    let calls = stats::collect_calls(module, &trace);

    BenchResult {
        stats,
        regions,
        blocks,
        calls,
        outcome,
    }
}
