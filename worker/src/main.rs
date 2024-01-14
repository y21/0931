use std::time::SystemTime;

use anyhow::anyhow;
use anyhow::bail;
use dash_compiler::FunctionCompiler;
use dash_decompiler::decompile;
use dash_middle::interner::StringInterner;
use dash_middle::parser::error::IntoFormattableErrors;
use dash_optimizer::OptLevel;
use dash_vm::eval::EvalError;
use dash_vm::gc::persistent::Persistent;
use dash_vm::params::VmParams;
use dash_vm::value::ops::conversions::ValueConversion;
use dash_vm::value::root_ext::RootOkExt;
use dash_vm::value::Root;
use dash_vm::value::Value;
use dash_vm::Vm;
use ipc2_worker::Job;
use shared::ClientMessage;
use shared::HostMessage;
use tokio::net::UnixStream;

fn fmt_value(inspect: &Persistent, value: Value, vm: &mut Vm) -> anyhow::Result<String> {
    let sc = &mut vm.scope();
    let result = match inspect
        .apply(sc, Value::undefined(), vec![value])
        .root_ok(sc)
    {
        Ok(v) => v,
        Err(_) => bail!("inspect function threw an exception"),
    };

    match result.to_js_string(sc) {
        Ok(v) => Ok(v.res(sc).to_owned()),
        Err(_) => Err(anyhow!("failed to convert inspected value to a string")),
    }
}

fn shared_opt_to_dash_opt(opt: shared::Opt) -> OptLevel {
    match opt {
        shared::Opt::None => OptLevel::None,
        shared::Opt::Basic => OptLevel::Basic,
        shared::Opt::Aggressive => OptLevel::Aggressive,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let mut rx = ipc2_worker::connect::<UnixStream, HostMessage, ClientMessage>().await?;

    while let Some(job) = rx.recv().await {
        match job {
            Job::Bidirectional { data, tx } => match data {
                HostMessage::DumpIr(code, opt) => {
                    let mut interner = StringInterner::new();
                    let output = match FunctionCompiler::compile_str(
                        &mut interner,
                        &code,
                        shared_opt_to_dash_opt(opt),
                    ) {
                        Ok(v) => {
                            decompile(&interner, &v.cp, &v.instructions).map_err(|x| x.to_string())
                        }
                        Err(err) => Err(err.formattable(&code, true).to_string()),
                    };

                    if tx.send(ClientMessage::EvalResponse(output)).is_err() {
                        tracing::error!("failed to respond to job!")
                    }
                }
                HostMessage::Eval(code, opt) => {
                    let params = VmParams::default()
                        .set_math_random_callback(|_| Ok(rand::random()))
                        .set_time_millis_callback(|_| {
                            Ok(SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap()
                                .as_millis()
                                .try_into()
                                .unwrap())
                        });
                    let mut vm = Vm::new(params);
                    let scope = &mut vm.scope();
                    let inspect = {
                        const INSPECT_CODE: &str = include_str!("../js/inspect.js");
                        let Value::Object(inspect) = scope
                            .eval(INSPECT_CODE, shared_opt_to_dash_opt(opt))
                            .unwrap()
                            .root(scope)
                        else {
                            unreachable!()
                        };

                        Persistent::new(scope, inspect)
                    };

                    let output = match scope.eval(&code, shared_opt_to_dash_opt(opt)) {
                        Ok(v) | Err(EvalError::Exception(v)) => {
                            fmt_value(&inspect, v.root(scope), scope).map_err(|err| err.to_string())
                        }
                        Err(EvalError::Middle(middle)) => {
                            Err(middle.formattable(&code, true).to_string())
                        }
                    };

                    if tx.send(ClientMessage::EvalResponse(output)).is_err() {
                        tracing::error!("failed to respond to job!")
                    }
                }
            },
            Job::Unidirectional { .. } => unreachable!("there are no unidirectional messages"),
        }
    }

    Ok(())
}
