use rustc_hash::FxHashMap;
use rustdoc_types::Crate;
use rustdoc_types::Enum;
use rustdoc_types::Function;
use rustdoc_types::GenericArg;
use rustdoc_types::GenericArgs;
use rustdoc_types::Id;
use rustdoc_types::Impl;
use rustdoc_types::Import;
use rustdoc_types::Item;
use rustdoc_types::ItemEnum;
use rustdoc_types::Module;
use rustdoc_types::Path;
use rustdoc_types::Primitive;
use rustdoc_types::Struct;
use rustdoc_types::StructKind;
use rustdoc_types::Trait;
use rustdoc_types::Type;
use rustdoc_types::Union;
use rustdoc_types::VariantKind;
use tracing::debug;
use tracing::info;

/// Crate context.
#[derive(Debug)]
pub struct CrateCtxt {
    pub krate: Crate,
    // /// Maps method id to parent item id (struct/enum/module)
    // /// and item (struct/enum/module) id to parent module id (module)
    // pub child_to_parent: FxHashMap<Id, Id>,
    // /// Maps method to impl block
    // pub method_to_impl: FxHashMap<Id, Id>,
}

macro_rules! expect_item_kind {
    ($self:expr, $id:expr, $p:pat => $r:expr) => {{
        let item = &$self.krate.index[$id];
        (
            item,
            match &item.inner {
                $p => $r,
                o => panic!("expected {}, got {o:?}", stringify!($p)),
            },
            item.name.as_deref().unwrap(),
        )
    }};
}
macro_rules! expect_item_kind_unnamed {
    ($self:expr, $id:expr, $p:pat => $r:expr) => {{
        let item = &$self.krate.index[$id];
        (
            item,
            match &item.inner {
                $p => $r,
                o => panic!("expected {}, got {o:?}", stringify!($p)),
            },
        )
    }};
}

impl CrateCtxt {
    pub fn expect_module(&self, id: &Id) -> (&Item, &Module, &str) {
        expect_item_kind!(self, id, ItemEnum::Module(m) => m)
    }

    pub fn expect_union(&self, id: &Id) -> (&Item, &Union, &str) {
        expect_item_kind!(self, id, ItemEnum::Union(u) => u)
    }

    pub fn expect_import(&self, id: &Id) -> (&Item, &Import) {
        expect_item_kind_unnamed!(self, id, ItemEnum::Import(i) => i)
    }

    pub fn expect_struct(&self, id: &Id) -> (&Item, &Struct, &str) {
        expect_item_kind!(self, id, ItemEnum::Struct(s) => s)
    }

    pub fn expect_function(&self, id: &Id) -> (&Item, &Function, &str) {
        expect_item_kind!(self, id, ItemEnum::Function(f) => f)
    }

    pub fn expect_struct_field(&self, id: &Id) -> (&Item, &Type, &str) {
        expect_item_kind!(self, id, ItemEnum::StructField(t) => t)
    }

    pub fn expect_impl(&self, id: &Id) -> (&Item, &Impl) {
        expect_item_kind_unnamed!(self, id, ItemEnum::Impl(i) => i)
    }

    pub fn run_visitor(&self, out: &mut Output) {
        let root = &self.krate.root;
        self.visit_module(root, out);
    }

    fn visit_module(&self, id: &Id, out: &mut Output) {
        let (_, module, name) = self.expect_module(id);
        out.segment_stack.push(name.to_owned());
        println!("{:?}", out.segment_stack);

        for item in &module.items {
            self.visit_item(item, out);
        }

        out.segment_stack.pop();
    }

    fn visit_item(&self, id: &Id, out: &mut Output) {
        let item = &self.krate.index[id];
        match &item.inner {
            ItemEnum::Module(_) => self.visit_module(id, out),
            ItemEnum::ExternCrate { .. } => {}
            ItemEnum::Import(_) => self.visit_import(id, out), // ???
            ItemEnum::Union(_) => self.visit_union(id, out),
            ItemEnum::Struct(_) => self.visit_struct(id, out),
            ItemEnum::StructField(_) => unreachable!("struct fields should be read by struct"),
            ItemEnum::Enum(_) => self.visit_enum(id, out),
            ItemEnum::Variant(_) => unreachable!("enum variants should be read by enum"),
            ItemEnum::Function(_) => self.visit_function(id, out),
            ItemEnum::Trait(_) => self.visit_trait(id, out),
            ItemEnum::TraitAlias(_) => todo!(), // ???
            ItemEnum::Impl(_) => todo!(),       // ???
            ItemEnum::Typedef(_) => self.visit_typedef(id, out),
            ItemEnum::OpaqueTy(_) => {} // ???
            ItemEnum::Constant(_) => {}
            ItemEnum::Static(_) => {} // maybe?
            ItemEnum::ForeignType => {}
            ItemEnum::Macro(_) => self.visit_macro(id, out),
            ItemEnum::ProcMacro(_) => self.visit_proc_macro(id, out),
            ItemEnum::Primitive(_) => {} // can this even appear here?
            ItemEnum::AssocConst { .. } => {}
            ItemEnum::AssocType { .. } => {}
        }
    }

    fn visit_union(&self, id: &Id, out: &mut Output) {
        let (item, union, name) = self.expect_union(id);
        self.visit_impls(&union.impls, out);
    }

    fn visit_struct(&self, id: &Id, out: &mut Output) {
        let (_, strukt, name) = self.expect_struct(id);
        let mut doc_string = String::new();
        self.write_struct(&mut doc_string, strukt, name);
        let path = out.to_path_string_with(name);
        out.index.push(path.into());
        out.docs.push(doc_string.into());
    }

    fn visit_enum(&self, id: &Id, out: &mut Output) {}
    fn visit_function(&self, id: &Id, out: &mut Output) {
        let mut doc_string = String::new();
        let (item, function, name) = self.expect_function(id);
        self.write_function(&mut doc_string, function, name);
        let path = out.to_path_string_with(name);
        out.index.push(path.into());
        out.docs.push(doc_string.into());
    }
    fn visit_trait(&self, id: &Id, out: &mut Output) {}
    fn visit_typedef(&self, id: &Id, out: &mut Output) {}
    fn visit_macro(&self, id: &Id, out: &mut Output) {}
    fn visit_proc_macro(&self, id: &Id, out: &mut Output) {}
    fn visit_import(&self, id: &Id, out: &mut Output) {
        let (item, import) = self.expect_import(id);
        if let Some(id) = &import.id {
            // Some imports are not part of the same crate or not even resolved at all
            // Let's check if it exists in the index before visiting to avoid a panic
            if self.krate.index.contains_key(id) {
                self.visit_item(id, out);
            }
        }
    }

    fn visit_fields(&self, id: &Id, out: &mut Output) {}
    fn visit_field(&self, id: &Id, out: &mut Output) {}
    fn visit_impl(&self, id: &Id, out: &mut Output) {}
    fn visit_impls(&self, ids: &[Id], out: &mut Output) {}

    fn write_struct(
        &self,
        out: &mut String,
        Struct {
            kind,
            generics,
            impls,
        }: &Struct,
        name: &str,
    ) {
        out.push_str("struct ");
        out.push_str(name);
        match kind {
            StructKind::Plain {
                fields,
                fields_stripped,
            } => {
                out.push_str(" {\n");
                for field in fields.iter() {
                    out.push_str("  ");
                    let (_, ty, name) = self.expect_struct_field(field);
                    out.push_str(name);
                    out.push_str(": ");
                    self.write_type(out, ty);
                    out.push_str(",\n");
                }

                if *fields_stripped {
                    out.push_str("  // private fields ommitted\n");
                }

                out.push('}');
                out.push_str("\nimpl ");
                out.push_str(name);
                out.push_str(" {\n");

                for id in impls {
                    let (_, imp) = self.expect_impl(id);
                    if imp.trait_.is_some() {
                        // for now...
                        continue;
                    }

                    for id in &imp.items {
                        let item = &self.krate.index[id];
                        match &item.inner {
                            ItemEnum::Function(f) => {
                                out.push_str("  ");
                                self.write_function(out, f, item.name.as_deref().unwrap());
                                out.push_str(";\n");
                            }
                            other => println!("??: {other:?}"),
                        }
                    }
                }

                out.push('}');
            }
            StructKind::Unit => out.push(';'),
            StructKind::Tuple(_) => {} // TODO!
            other => panic!("unsupported struct kind: {other:?}"),
        }
    }

    fn write_function(
        &self,
        out: &mut String,
        Function {
            decl,
            generics,
            header,
            has_body: _,
        }: &Function,
        name: &str,
    ) {
        if header.async_ {
            out.push_str("async ");
        }
        if header.const_ {
            out.push_str("const ");
        }
        if header.unsafe_ {
            out.push_str("unsafe ");
        }
        out.push_str("fn ");
        out.push_str(name);
        if !generics.params.is_empty() {
            out.push('<');
            for (i, param) in generics.params.iter().enumerate() {
                if i != 0 {
                    out.push_str(", ");
                }
                out.push_str(&param.name);
            }
            out.push('>');
        }

        out.push('(');
        for (i, (name, ty)) in decl.inputs.iter().enumerate() {
            if i != 0 {
                out.push_str(", ");
            }

            if name == "self" {
                match ty {
                    Type::BorrowedRef { mutable, .. } if *mutable => out.push_str("&mut "),
                    Type::BorrowedRef { mutable, .. } if !*mutable => out.push('&'),
                    _ => {}
                }
                out.push_str("self");
            } else {
                out.push_str(name);
                out.push_str(": ");
                self.write_type(out, ty);
            }
        }
        out.push(')');
        if let Some(output) = &decl.output {
            out.push_str(" -> ");
            self.write_type(out, output);
        }
    }

    fn write_type(&self, out: &mut String, ty: &Type) {
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
                self.write_type(out, type_);
            }
            Type::Tuple(tup) => {
                out.push('(');
                for ty in tup.iter() {
                    out.push_str(", ");
                    self.write_type(out, ty);
                }
                out.push(')');
            }
            Type::Array { type_, len } => {
                out.push('[');
                self.write_type(out, type_);
                out.push_str("; ");
                out.push_str(len);
                out.push(']');
            }
            Type::DynTrait(dyn_trait) => {
                out.push_str("dyn ");
                // TODO: lifetime
                for (index, tr) in dyn_trait.traits.iter().enumerate() {
                    if index != 0 {
                        out.push_str(" + ");
                    }
                    out.push_str(&tr.trait_.name);
                }
            }
            Type::FunctionPointer(_fp) => {
                // TODO: actually implement it
                out.push_str("fn()");
            }
            Type::Generic(name) => {
                out.push_str(name);
            }
            Type::Primitive(prim) => {
                out.push_str(prim);
            }
            Type::Slice(slice) => {
                out.push('[');
                self.write_type(out, slice);
                out.push(']');
            }
            Type::RawPointer { mutable, type_ } => {
                out.push('*');
                match *mutable {
                    true => out.push_str("mut "),
                    false => out.push_str("const "),
                }
                self.write_type(out, type_);
            }
            Type::ResolvedPath(path) => {
                out.push_str(&path.name);

                if let Some(args) = &path.args {
                    match &**args {
                        GenericArgs::AngleBracketed { args, .. } => {
                            if !args.is_empty() {
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
                                        GenericArg::Type(ty) => self.write_type(out, ty),
                                        GenericArg::Const(c) => {
                                            panic!("const generics not supported ({c:?})")
                                        }
                                    }
                                }
                                out.push('>');
                            }
                        }
                        GenericArgs::Parenthesized { inputs, output } => {
                            if !inputs.is_empty() {
                                out.push('(');
                                for (i, arg) in inputs.iter().enumerate() {
                                    if i != 0 {
                                        out.push_str(", ");
                                    }
                                    self.write_type(out, arg);
                                }
                                out.push(')');
                            }
                            if let Some(output) = output {
                                out.push_str(" -> ");
                                self.write_type(out, output);
                            }
                        }
                    }
                }
            }
            _ => panic!("unknown type {:?}", ty),
        }
    }
}

#[derive(Debug, Default)]
pub struct Output {
    segment_stack: Vec<String>,
    // indices must be kept in sync, i.e.
    // self.index[5] must correspond to the docs in self.docs[5]
    pub index: Vec<Box<str>>,
    pub docs: Vec<Box<str>>,
}

impl Output {
    fn to_path_string_with(&self, last: &str) -> String {
        let mut path = self.segment_stack.join("::");
        path += "::";
        path += last;
        path
    }
}

impl CrateCtxt {}

/// Doc context. Can carry additional mutable state.
#[derive(Debug)]
pub struct DocCtxt {
    pub ctxt: CrateCtxt,
    pub out: Output,
}

// fn register_many_relations(children: &[Id], parent: &Id, out: &mut FxHashMap<Id, Id>) {
//     for child in children {
//         out.insert(child.clone(), parent.clone());
//     }
// }

// fn register_many_optional_relations(
//     children: &[Option<Id>],
//     parent: &Id,
//     out: &mut FxHashMap<Id, Id>,
// ) {
//     for child in children.iter().filter_map(|x| x.clone()) {
//         out.insert(child, parent.clone());
//     }
// }

// impl CrateCtxt {
// #[tracing::instrument(skip(self))]
// pub fn compute_child_parent_relations(&mut self) {
//     info!("compute child-parent relations");
//     for (id, item) in self.krate.index.iter() {
//         match &item.inner {
//             ItemEnum::Union(Union { fields, impls, .. }) => {
//                 register_many_relations(fields, id, &mut self.child_to_parent);
//                 register_many_relations(impls, id, &mut self.child_to_parent);
//             }
//             ItemEnum::Struct(Struct { kind, impls, .. }) => {
//                 register_many_relations(impls, id, &mut self.child_to_parent);
//                 match kind {
//                     StructKind::Unit => {}
//                     StructKind::Tuple(t) => {
//                         register_many_optional_relations(t, id, &mut self.child_to_parent)
//                     }
//                     StructKind::Plain { fields, .. } => {
//                         register_many_relations(fields, id, &mut self.child_to_parent)
//                     }
//                 }
//             }
//             ItemEnum::StructField(_) => {}
//             ItemEnum::Enum(Enum {
//                 variants, impls, ..
//             }) => {
//                 register_many_relations(impls, id, &mut self.child_to_parent);
//                 register_many_relations(variants, id, &mut self.child_to_parent);
//             }
//             ItemEnum::Variant(v) => match &v.kind {
//                 VariantKind::Plain => {}
//                 VariantKind::Tuple(t) => {
//                     register_many_optional_relations(t, id, &mut self.child_to_parent)
//                 }
//                 VariantKind::Struct { fields, .. } => {
//                     register_many_relations(fields, id, &mut self.child_to_parent)
//                 }
//             },
//             ItemEnum::Function(_) => {}
//             ItemEnum::Trait(Trait {
//                 items,
//                 implementations,
//                 ..
//             }) => {
//                 register_many_relations(items, id, &mut self.child_to_parent);
//                 register_many_relations(implementations, id, &mut self.child_to_parent);
//             }
//             ItemEnum::TraitAlias(_) => {}
//             ItemEnum::Impl(Impl { items, .. }) => {
//                 register_many_relations(items, id, &mut self.child_to_parent);
//                 register_many_relations(items, id, &mut self.method_to_impl);
//             }
//             ItemEnum::Typedef(_) => {}
//             ItemEnum::OpaqueTy(_) => {}
//             ItemEnum::Constant(_) => {}
//             ItemEnum::Static(_) => {}
//             ItemEnum::ForeignType => {}
//             ItemEnum::Macro(_) => {}
//             ItemEnum::ProcMacro(_) => todo!(),
//             ItemEnum::Primitive(Primitive { impls, .. }) => {
//                 register_many_relations(impls, id, &mut self.child_to_parent);
//             }
//             ItemEnum::AssocConst { .. } => {}
//             ItemEnum::AssocType { .. } => {}
//             ItemEnum::Module(Module { items, .. }) => {
//                 println!("{:?} {:?}", item.name, item);
//                 register_many_relations(items, id, &mut self.child_to_parent);
//             }
//             ItemEnum::ExternCrate { .. } => {}
//             // ItemEnum::Import(Import {
//             //     id: Some(id),
//             //     glob: false,
//             //     ..
//             // }) => {
//             //     self.child_to_parent.insert(id.clone(), item.id.clone());
//             // }
//             ItemEnum::Import(_) => {}
//         }
//     }
//     info!("finished computing child-parent relations");
//     debug!("child_to_parent: {}", self.child_to_parent.len());
//     debug!("method_to_impl: {}", self.method_to_impl.len());
// }

// #[tracing::instrument(skip(self))]
// pub fn populate_output(&mut self, out: &mut Output) {
//     for (_id, item) in self.krate.index.iter() {
//         match &item.inner {
//             ItemEnum::Function(_) | ItemEnum::Struct(_) | ItemEnum::Enum(_) => {
//                 let name = self.to_segmented_path(item);
//                 // println!("{name}");
//                 // let parent = self.child_to_parent.get(&item.id);
//                 // match parent {
//                 //     Some(x) => {
//                 //         // println!("{:?}", &item.name, &self.krate.index[x]);
//                 //         // println!("OK")
//                 //         let parent = &self.krate.index[x];
//                 //         let parent_inner = match &parent.inner {
//                 //             ItemEnum::Module(m) => parent.name.as_deref().unwrap(),
//                 //             ItemEnum::Impl(i) => {
//                 //                 println!("impl! {:?}", i.for_);
//                 //                 ""
//                 //             }
//                 //             _ => "",
//                 //         };

//                 //         // println!("{:?} (parent: {:?})", item.name, parent.name);
//                 //     }
//                 //     None => {}
//                 // };
//                 // // println!("{:?}", &item.name);
//                 //
//             }
//             ItemEnum::Constant(_) => {}
//             _ => {}
//         }
//     }
// }

// /// NOTE: this `&Item` must have a name (calling this with `Function`s for example is fine)
// #[tracing::instrument(skip(self))]
// pub fn to_segmented_path(&self, item: &Item) -> String {
//     // Start pushing in reverse order
//     let this = item.name.clone().unwrap();
//     let mut segments = vec![this];

//     let mut next_id = self.child_to_parent.get(&item.id);
//     while let Some(id) = next_id {
//         let parent = &self.krate.index[id];
//         // println!("{:?}", parent);
//         match &parent.inner {
//             ItemEnum::Module(_) | ItemEnum::Struct(_) | ItemEnum::Enum(_) => {
//                 segments.push(parent.name.clone().unwrap());
//             }
//             ItemEnum::Impl(Impl {
//                 for_: Type::Primitive(prim),
//                 ..
//             }) => {
//                 segments.push(prim.clone());
//             }
//             ItemEnum::Impl(_) => {}
//             ItemEnum::Primitive(_) => {}
//             ItemEnum::Trait(_) => {}
//             ItemEnum::Import(Import { name, source, .. }) => {
//                 segments.push(name.clone());
//                 // let next = self.child_to_parent.get(id);
//                 // if let Some(next) = next {
//                 //     next_id = self.child_to_parent.get(next);
//                 // }
//                 // continue;
//                 // Intentionally do nothing. The parent of this `use` is the actual referenced item.
//             }
//             other => todo!("{other:?}"),
//         };
//         next_id = self.child_to_parent.get(id);
//     }

//     segments.reverse();
//     // if segments.len() == 1 {
//     //     println!("{:?}", segments);
//     // }
//     segments.join("::")
// }
// }
