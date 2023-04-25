use std::time::Duration;

use anyhow::bail;
use anyhow::Context;
use human_size::Byte;
use human_size::Megabyte;
use human_size::SpecificSize;
use ipc2_host::workerset::TimeoutAction;
use itertools::Itertools;
use poise::samples::HelpConfiguration;
use poise::CodeBlock;
use rustdoc_types::Function;
use rustdoc_types::GenericArg;
use rustdoc_types::GenericArgs;
use rustdoc_types::ItemEnum;
use rustdoc_types::Type;
use shared::ClientMessage;
use shared::HostMessage;
use std::fmt::Write;
use sublime_fuzzy::best_match;
use sysinfo::CpuExt;
use sysinfo::SystemExt;

use crate::godbolt;
use crate::playground;
use crate::state::State;
use crate::util;
use crate::util::CodeBlockOrRest;
use crate::PoiseContext;

/// Executes a Rust codeblock
///
/// The code can simply be an expression and the bot will automatically
/// wrap it in a main function and a print statement.
#[poise::command(prefix_command, track_edits, broadcast_typing)]
pub async fn rust(cx: PoiseContext<'_>, block: CodeBlockOrRest) -> anyhow::Result<()> {
    let response = playground::run_code(&cx.data().reqwest, block.0).await?;

    cx.say(util::codeblock(util::strip_header_stderr(
        &response.output(),
    )))
    .await?;

    Ok(())
}

/// Benchmarks two Rust codeblocks to see which one runs faster
#[poise::command(prefix_command, track_edits, broadcast_typing)]
pub async fn bench(
    cx: PoiseContext<'_>,
    block1: CodeBlock,
    block2: CodeBlock,
) -> anyhow::Result<()> {
    let response = playground::bench_code(&cx.data().reqwest, block1.code, block2.code).await?;

    cx.say(util::codeblock(util::strip_header_stderr(
        &response.output(),
    )))
    .await?;

    Ok(())
}

/// Runs a codeblock under miri, an interpreter that checks for memory errors
#[poise::command(prefix_command, track_edits, broadcast_typing)]
pub async fn miri(cx: PoiseContext<'_>, block: CodeBlockOrRest) -> anyhow::Result<()> {
    let response = playground::run_miri(&cx.data().reqwest, block.0).await?;
    cx.say(util::codeblock(util::strip_header_stderr(
        &response.output(),
    )))
    .await?;

    Ok(())
}

/// Help me
#[poise::command(prefix_command, track_edits)]
pub async fn help(cx: PoiseContext<'_>, command: Option<String>) -> anyhow::Result<()> {
    let config = HelpConfiguration {
        extra_text_at_bottom:
            "You can edit your message to the bot and the bot will edit its response",
        ..Default::default()
    };

    poise::builtins::help(cx, command.as_deref(), config).await?;
    Ok(())
}

/// Compile a codeblock and get the assembly
#[poise::command(prefix_command, track_edits)]
pub async fn asm(cx: PoiseContext<'_>, blocks: Vec<CodeBlock>) -> anyhow::Result<()> {
    let mut output = String::new();

    for block in blocks {
        let out = godbolt::get_asm(&cx.data().reqwest, block.code).await?;
        output.push_str(&util::codeblock(&out.0));
    }

    cx.say(&output).await?;
    Ok(())
}

/// Compile two codeblocks and diff them
#[poise::command(prefix_command, track_edits)]
pub async fn asmdiff(
    cx: PoiseContext<'_>,
    block1: CodeBlock,
    block2: CodeBlock,
) -> anyhow::Result<()> {
    let State { reqwest, .. } = &**cx.data();
    let response1 = godbolt::get_asm(reqwest, block1.code).await?;
    let response2 = godbolt::get_asm(reqwest, block2.code).await?;

    cx.say(util::codeblock_with_lang(
        "diff",
        &response1.diff(response2),
    ))
    .await?;
    Ok(())
}

const MAX_TIME: Duration = Duration::from_secs(5);

/// Executes JavaScript code
#[poise::command(prefix_command, track_edits)]
pub async fn js(cx: PoiseContext<'_>, block: CodeBlockOrRest) -> anyhow::Result<()> {
    let CodeBlockOrRest(code) = block;
    tracing::info!(%code, "Send JS code to worker");

    let ClientMessage::EvalResponse(message) = cx
        .data()
        .workers
        .send_timeout(HostMessage::Eval(code), MAX_TIME, TimeoutAction::Restart)
        .await?;

    cx.say(util::codeblock_with_lang(
        "js",
        match &message {
            Ok(x) => x,
            Err(x) => x,
        },
    ))
    .await?;

    Ok(())
}

#[poise::command(prefix_command, track_edits)]
pub async fn info(cx: PoiseContext<'_>) -> anyhow::Result<()> {
    let output = {
        let temperature = util::get_temp()?
            .map(|t| t.to_string())
            .unwrap_or_else(|| "<unsupported>".into());

        let mut sys = cx.data().system.lock().unwrap();
        sys.refresh_all();

        let mut output = format!("```\nTemperature: {temperature}\n");

        for (id, cpu) in sys.cpus().iter().enumerate() {
            let _ = writeln!(output, "CPU #{id}: {:.2}%", cpu.cpu_usage());
        }

        let fmt_size = |bytes| {
            SpecificSize::new(bytes as f64, Byte)
                .unwrap()
                .into::<Megabyte>()
        };

        let total = fmt_size(sys.total_memory());
        let avail = fmt_size(sys.free_memory());
        let ratio = (avail.to_bytes() as f64 / total.to_bytes() as f64) * 100.0;

        let _ = writeln!(
            output,
            "Memory: {:.2}/{:.2} ({:.2}%)\n```",
            avail, total, ratio
        );

        output
    };

    cx.say(output).await?;

    Ok(())
}

#[poise::command(prefix_command, track_edits)]
pub async fn fuzzy(cx: PoiseContext<'_>, query: String, search: String) -> anyhow::Result<()> {
    let result = best_match(&query, &search).context("No match!")?;

    let message = format!(
        "Score: {} \n\
        Matched indices: {}
    ",
        result.score(),
        result.matched_indices().join(", ")
    );

    cx.say(message).await?;
    Ok(())
}

#[poise::command(prefix_command, track_edits)]
pub async fn docs(cx: PoiseContext<'_>, query: String) -> anyhow::Result<()> {
    let item = cx.data().docs.find(&query).context("Nothing found!")?;

    // unwrap is safe, checked by Docs::find
    let name = item.name.as_deref().unwrap();

    // TODO: consider visibility
    let mut response = String::from("```rs\npub ");

    match &item.inner {
        ItemEnum::Function(Function {
            decl,
            generics,
            header,
            has_body: _,
        }) => {
            if header.async_ {
                response.push_str("async ");
            }
            if header.const_ {
                response.push_str("const ");
            }
            if header.unsafe_ {
                response.push_str("unsafe ");
            }
            response.push_str("fn ");
            response.push_str(name);
            if !generics.params.is_empty() {
                response.push('<');
                for (i, param) in generics.params.iter().enumerate() {
                    if i != 0 {
                        response.push_str(", ");
                    }
                    response.push_str(&param.name);
                }
                response.push('>');
            }

            response.push('(');
            for (i, (name, ty)) in decl.inputs.iter().enumerate() {
                if i != 0 {
                    response.push_str(", ");
                }

                response.push_str(name);
                response.push_str(": ");
                type_to_string(&mut response, ty)?;
            }
            response.push(')');
            if let Some(out) = &decl.output {
                response.push_str(" -> ");
                type_to_string(&mut response, out)?;
            }
        }
        _ => bail!("unsupported item type: `{:?}`", item.inner),
    }
    response.push_str("\n```\n");

    if let Some(docs) = &item.docs {
        response.extend(docs.chars().take(500));

        if docs.len() > 500 {
            response.push('…');
            response.push_str(" [Read more](<https://google.com>)\n");
        }
    }

    if !item.links.is_empty() {
        response.push_str("Go To: ");
        for name in item.links.keys() {
            response.push_str(&format!("[{}](<https://google.com>)  ", name));
        }
    }

    cx.say(response).await?;

    Ok(())
}

fn type_to_string(out: &mut String, ty: &Type) -> anyhow::Result<()> {
    match ty {
        Type::BorrowedRef {
            lifetime,
            mutable,
            type_,
        } => {
            out.push_str(&format!(
                "&{}{}",
                lifetime.as_deref().unwrap_or(""),
                if *mutable { "mut " } else { "" },
            ));
            type_to_string(out, type_)?;
        }
        Type::Generic(name) => {
            out.push_str(name);
        }
        Type::Primitive(prim) => {
            out.push_str(prim);
        }
        Type::Slice(slice) => {
            out.push('[');
            type_to_string(out, slice)?;
            out.push(']');
        }
        Type::RawPointer { mutable, type_ } => {
            out.push('*');
            match *mutable {
                true => out.push_str("mut "),
                false => out.push_str("const "),
            }
            type_to_string(out, type_)?;
        }
        Type::ResolvedPath(path) => {
            out.push_str(&path.name);

            if let Some(args) = &path.args {
                match &**args {
                    GenericArgs::AngleBracketed { args, .. } => {
                        out.push('<');
                        for (i, arg) in args.iter().enumerate() {
                            if i != 0 {
                                out.push_str(", ");
                            }

                            match arg {
                                GenericArg::Lifetime(lt) => {
                                    out.push_str(&format!("'{}", lt));
                                }
                                GenericArg::Infer => out.push('_'),
                                GenericArg::Type(ty) => type_to_string(out, ty)?,
                                GenericArg::Const(c) => {
                                    bail!("const generics not supported ({c:?})")
                                }
                            }
                        }
                        out.push('>');
                    }
                    GenericArgs::Parenthesized { inputs, output } => {
                        out.push('(');
                        for (i, arg) in inputs.iter().enumerate() {
                            if i != 0 {
                                out.push_str(", ");
                            }
                            type_to_string(out, arg)?;
                        }
                        out.push(')');
                        if let Some(output) = output {
                            out.push_str(" -> ");
                            type_to_string(out, output)?;
                        }
                    }
                }
            }
        }
        _ => bail!("unknown type {:?}", ty),
    }
    Ok(())
}
