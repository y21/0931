// We have a command for running code, similar to the regular rust playground,
// but also with access to rustc internals.
#![feature(rustc_private, let_chains, box_patterns)]
#![allow(dead_code, unused_imports)]

extern crate rustc_ast_pretty;
extern crate rustc_driver;
extern crate rustc_error_codes;
extern crate rustc_errors;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use std::{path, process, str};

use rustc_ast_pretty::pprust::item_to_string;
use rustc_errors::registry;
use rustc_hir as hir;
use rustc_middle::{
    hir::map::Map,
    mir,
    ty::{self, Ty, TyCtxt, TypeckResults},
};
use rustc_session::config::{self, CheckCfg};
use rustc_span::{source_map, Span};

struct Context<'tcx> {
    tcx: TyCtxt<'tcx>,
    typeck_results: &'tcx TypeckResults<'tcx>,
}

fn main() {
    let out = process::Command::new("rustc")
        .arg("--print=sysroot")
        .current_dir(".")
        .output()
        .unwrap();
    let sysroot = str::from_utf8(&out.stdout).unwrap().trim();
    let config = rustc_interface::Config {
        opts: config::Options {
            maybe_sysroot: Some(path::PathBuf::from(sysroot)),
            ..config::Options::default()
        },
        input: config::Input::Str {
            name: source_map::FileName::Custom("main.rs".to_string()),
            input: r####"/*{{input}}*/"####.to_string(),
        },
        crate_cfg: rustc_hash::FxHashSet::default(),
        crate_check_cfg: CheckCfg::default(),
        output_dir: None,
        output_file: None,
        file_loader: None,
        locale_resources: rustc_driver::DEFAULT_LOCALE_RESOURCES,
        lint_caps: rustc_hash::FxHashMap::default(),
        parse_sess_created: None,
        register_lints: None,
        override_queries: None,
        make_codegen_backend: None,
        registry: registry::Registry::new(rustc_error_codes::DIAGNOSTICS),
        ice_file: None,
    };
    rustc_interface::run_compiler(config, |compiler| {
        compiler.enter(|queries| {
            let ast = queries.parse().unwrap().get_mut().clone();
            queries.global_ctxt().unwrap().enter(|tcx| {
                let hir = tcx.hir();
                let krate = hir.krate();
                println!("{:#?}", { /*{{code}}*/ });
            });
        });
    });
}

fn for_each_item_in_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    cb: impl FnMut(TyCtxt<'tcx>, &'tcx hir::Item<'tcx>),
) {
    struct ItemVisitor<'tcx, F> {
        tcx: TyCtxt<'tcx>,
        cb: F,
    }
    impl<'tcx, F> hir::intravisit::Visitor<'tcx> for ItemVisitor<'tcx, F>
    where
        F: FnMut(TyCtxt<'tcx>, &'tcx hir::Item<'tcx>),
    {
        type NestedFilter = rustc_middle::hir::nested_filter::All;
        fn nested_visit_map(&mut self) -> Self::Map {
            self.tcx.hir()
        }
        fn visit_item(&mut self, item: &'tcx hir::Item<'tcx>) {
            (self.cb)(self.tcx, item);
            hir::intravisit::walk_item(self, item);
        }
    }

    let mut vis = ItemVisitor { tcx, cb };
    let hir = tcx.hir();
    for item in hir.items() {
        hir::intravisit::Visitor::visit_item(&mut vis, hir.item(item));
    }
}

fn for_each_expr_in_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    cb: impl FnMut(Context<'tcx>, &rustc_hir::Expr<'tcx>),
) {
    struct ExprVisitor<'tcx, F> {
        tcx: TyCtxt<'tcx>,
        // stack
        typeck: Vec<&'tcx TypeckResults<'tcx>>,
        cb: F,
    }
    impl<'tcx, F> hir::intravisit::Visitor<'tcx> for ExprVisitor<'tcx, F>
    where
        F: FnMut(Context<'tcx>, &hir::Expr<'tcx>),
    {
        type NestedFilter = rustc_middle::hir::nested_filter::All;
        fn nested_visit_map(&mut self) -> Self::Map {
            self.tcx.hir()
        }
        fn visit_expr(&mut self, ex: &'tcx hir::Expr<'tcx>) {
            (self.cb)(
                Context {
                    tcx: self.tcx,
                    typeck_results: self.typeck.last().unwrap(),
                },
                ex,
            );
            hir::intravisit::walk_expr(self, ex);
        }
        fn visit_body(&mut self, b: &'tcx hir::Body<'tcx>) {
            let id = self.tcx.hir().body_owner_def_id(b.id());
            let results = self.tcx.typeck(id);
            self.typeck.push(results);
            hir::intravisit::walk_body(self, b);
            self.typeck.pop();
        }
    }

    let mut vis = ExprVisitor {
        tcx,
        cb,
        typeck: Vec::new(),
    };
    let hir = tcx.hir();
    for item in hir.items() {
        hir::intravisit::walk_item(&mut vis, hir.item(item));
    }
}
