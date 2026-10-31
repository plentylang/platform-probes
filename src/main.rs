use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature, types};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::error::Error;
use std::path::Path;
use std::process::Command;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn signature(
    module: &ObjectModule,
    params: &[cranelift_codegen::ir::Type],
    result: cranelift_codegen::ir::Type,
) -> Signature {
    let mut sig = module.make_signature();
    sig.params.extend(params.iter().copied().map(AbiParam::new));
    sig.returns.push(AbiParam::new(result));
    sig
}

fn sum_function(
    module: &mut ObjectModule,
    name: &str,
    sig: &Signature,
    linkage: Linkage,
) -> Result<FuncId> {
    let id = module.declare_function(name, linkage, sig)?;
    let mut ctx = module.make_context();
    ctx.func.signature = sig.clone();
    let mut frontend = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut frontend);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    b.seal_all_blocks();
    let args = b.block_params(entry).to_vec();
    let mut total = b.ins().iconst(types::I64, 0);
    for (i, arg) in args.into_iter().enumerate() {
        let weighted = b.ins().imul_imm_s(arg, (i + 1) as i64);
        total = b.ins().iadd(total, weighted);
    }
    b.ins().return_(&[total]);
    b.finalize(module.target_config());
    module.define_function(id, &mut ctx)?;
    Ok(id)
}

fn forwarding_function(
    module: &mut ObjectModule,
    name: &str,
    sig: &Signature,
    callee: FuncId,
) -> Result<()> {
    let id = module.declare_function(name, Linkage::Export, sig)?;
    let mut ctx = module.make_context();
    ctx.func.signature = sig.clone();
    let target = module.declare_func_in_func(callee, &mut ctx.func);
    let mut frontend = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut frontend);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    b.seal_all_blocks();
    let args = b.block_params(entry).to_vec();
    let call = b.ins().call(target, &args);
    let results = b.inst_results(call).to_vec();
    b.ins().return_(&results);
    b.finalize(module.target_config());
    module.define_function(id, &mut ctx)?;
    Ok(())
}

fn generate(path: &Path) -> Result<()> {
    let mut flags = settings::builder();
    // Native linkers (particularly Mach-O) require position-independent calls.
    flags.set("is_pic", "true")?;
    let isa = cranelift_native::builder()
        .map_err(std::io::Error::other)?
        .finish(settings::Flags::new(flags))?;
    println!(
        "native target: {}; C ABI: {:?}",
        isa.triple(),
        isa.default_call_conv()
    );
    let mut module = ObjectModule::new(ObjectBuilder::new(
        isa,
        "abi-probes",
        cranelift_module::default_libcall_names(),
    )?);
    let integers = signature(&module, &[types::I64; 10], types::I64);
    sum_function(&mut module, "probe_sum10", &integers, Linkage::Export)?;
    let callback = module.declare_function("host_sum10", Linkage::Import, &integers)?;
    forwarding_function(&mut module, "probe_callback10", &integers, callback)?;
    let mut tail_sig = integers.clone();
    tail_sig.call_conv = CallConv::Tail;
    let tail = sum_function(
        &mut module,
        "internal_tail_sum10",
        &tail_sig,
        Linkage::Local,
    )?;
    forwarding_function(&mut module, "probe_tail_adapter10", &integers, tail)?;

    let mixed_types = [types::I64, types::F64].repeat(6);
    let mixed = signature(&module, &mixed_types, types::F64);
    let mixed_id = module.declare_function("probe_mixed", Linkage::Export, &mixed)?;
    let mixed_callback = module.declare_function("host_mixed", Linkage::Import, &mixed)?;
    forwarding_function(&mut module, "probe_mixed_callback", &mixed, mixed_callback)?;
    let mut ctx = module.make_context();
    ctx.func.signature = mixed;
    let mut frontend = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut frontend);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    b.seal_all_blocks();
    let args = b.block_params(entry).to_vec();
    let mut total = b.ins().f64const(0.0);
    for (i, arg) in args.into_iter().enumerate() {
        let value = if i % 2 == 0 {
            b.ins().fcvt_from_sint(types::F64, arg)
        } else {
            arg
        };
        let weight = b.ins().f64const((i + 1) as f64);
        let term = b.ins().fmul(value, weight);
        total = b.ins().fadd(total, term);
    }
    b.ins().return_(&[total]);
    b.finalize(module.target_config());
    module.define_function(mixed_id, &mut ctx)?;

    let mut narrow = module.make_signature();
    narrow.params = vec![
        AbiParam::new(types::I8).sext(),
        AbiParam::new(types::I16).uext(),
    ];
    narrow.returns.push(AbiParam::new(types::I64));
    let id = module.declare_function("probe_narrow", Linkage::Export, &narrow)?;
    let callback = module.declare_function("host_narrow", Linkage::Import, &narrow)?;
    forwarding_function(&mut module, "probe_narrow_callback", &narrow, callback)?;
    let mut ctx = module.make_context();
    ctx.func.signature = narrow;
    let mut frontend = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut frontend);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    b.seal_all_blocks();
    let args = b.block_params(entry).to_vec();
    let signed = b.ins().sextend(types::I64, args[0]);
    let unsigned = b.ins().uextend(types::I64, args[1]);
    let result = b.ins().iadd(signed, unsigned);
    b.ins().return_(&[result]);
    b.finalize(module.target_config());
    module.define_function(id, &mut ctx)?;
    std::fs::write(path, module.finish().emit()?)?;
    Ok(())
}

fn checked(command: &mut Command) -> Result<()> {
    println!(
        "running: {:?} {:?}",
        command.get_program(),
        command.get_args().collect::<Vec<_>>()
    );
    let status = command.status()?;
    if !status.success() {
        return Err(format!("command failed: {status}").into());
    }
    Ok(())
}

fn main() -> Result<()> {
    let out = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "target/probes".into());
    std::fs::create_dir_all(&out)?;
    // Preserve ordinary Windows paths: MSVC rejects canonicalize verbatim prefixes.
    let out = std::path::absolute(out)?;
    let object = out.join(if cfg!(windows) {
        "probes.obj"
    } else {
        "probes.o"
    });
    generate(&object)?;
    let target = env!("PROBE_TARGET");
    let tool = cc::Build::new()
        .host(target)
        .target(target)
        .opt_level(0)
        .debug(true)
        .out_dir(&out)
        .cargo_metadata(false)
        .try_get_compiler()?;
    let exe = out.join(if cfg!(windows) {
        "abi-harness.exe"
    } else {
        "abi-harness"
    });
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("harness.c");
    let mut command = tool.to_command();
    if tool.is_like_msvc() {
        command
            .arg("/W4")
            .arg("/WX")
            .arg(&source)
            .arg(&object)
            .arg(format!("/Fe{}", exe.display()))
            .arg(format!("/Fo{}", out.join("harness.obj").display()));
    } else {
        command
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
            .arg(&source)
            .arg(&object)
            .arg("-o")
            .arg(&exe);
    }
    checked(&mut command)?;
    checked(&mut Command::new(exe))?;
    Ok(())
}
