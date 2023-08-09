use std::time::SystemTime;

use anyhow::anyhow;
use anyhow::bail;
use dash_vm::eval::EvalError;
use dash_vm::gc::persistent::Persistent;
use dash_vm::params::VmParams;
use dash_vm::value::object::Object;
use dash_vm::value::ops::abstractions::conversions::ValueConversion;
use dash_vm::value::Value;
use dash_vm::Vm;
use ipc2_worker::Job;
use shared::ClientMessage;
use shared::HostMessage;
use tokio::net::UnixStream;

fn fmt_value(
    inspect: &Persistent<dyn Object>,
    value: Value,
    vm: &mut Vm,
) -> anyhow::Result<String> {
    let sc = &mut vm.scope();
    let result = match inspect.apply(sc, Value::undefined(), vec![value]) {
        Ok(v) => v,
        Err(_) => bail!("inspect function threw an exception"),
    };

    match result.to_string(sc) {
        Ok(v) => Ok(String::from(&*v)),
        Err(_) => Err(anyhow!("failed to convert inspected value to a string")),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let mut rx = ipc2_worker::connect::<UnixStream, HostMessage, ClientMessage>().await?;

    while let Some(job) = rx.recv().await {
        match job {
            Job::Bidirectional { data, tx } => match data {
                HostMessage::Eval(code) => {
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
                            .eval(INSPECT_CODE, Default::default())
                            .unwrap()
                            .root(scope)
                        else {
                            unreachable!()
                        };

                        Persistent::new(scope, inspect)
                    };

                    let output = match scope.eval(&code, Default::default()) {
                        Ok(v) | Err(EvalError::Exception(v)) => {
                            fmt_value(&inspect, v.root(scope), scope).map_err(|err| err.to_string())
                        }
                        Err(err) => Err(err.to_string()),
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
