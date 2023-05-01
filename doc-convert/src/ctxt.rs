use itertools::Itertools;
use rustdoc_types::Crate;
use rustdoc_types::Enum;
use rustdoc_types::FnDecl;
use rustdoc_types::Function;
use rustdoc_types::GenericArg;
use rustdoc_types::GenericArgs;
use rustdoc_types::GenericBound;
use rustdoc_types::Generics;
use rustdoc_types::Header;
use rustdoc_types::Id;
use rustdoc_types::Impl;
use rustdoc_types::Import;
use rustdoc_types::Item;
use rustdoc_types::ItemEnum;
use rustdoc_types::Module;
use rustdoc_types::Path;
use rustdoc_types::Struct;
use rustdoc_types::StructKind;
use rustdoc_types::Type;
use rustdoc_types::Union;
use rustdoc_types::Variant;
use rustdoc_types::VariantKind;
use std::cmp::Ordering;
use std::fmt::Write;

/// Crate context.
#[derive(Debug)]
pub struct CrateCtxt {
    pub krate: Crate,
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

    pub fn expect_enum(&self, id: &Id) -> (&Item, &Enum, &str) {
        expect_item_kind!(self, id, ItemEnum::Enum(e) => e)
    }

    pub fn expect_function(&self, id: &Id) -> (&Item, &Function, &str) {
        expect_item_kind!(self, id, ItemEnum::Function(f) => f)
    }

    pub fn expect_struct_field(&self, id: &Id) -> (&Item, &Type, &str) {
        expect_item_kind!(self, id, ItemEnum::StructField(t) => t)
    }

    pub fn expect_unnamed_struct_field(&self, id: &Id) -> (&Item, &Type) {
        expect_item_kind_unnamed!(self, id, ItemEnum::StructField(t) => t)
    }

    pub fn expect_impl(&self, id: &Id) -> (&Item, &Impl) {
        expect_item_kind_unnamed!(self, id, ItemEnum::Impl(i) => i)
    }

    pub fn expect_variant(&self, id: &Id) -> (&Item, &Variant, &str) {
        expect_item_kind!(self, id, ItemEnum::Variant(v) => v)
    }

    pub fn run_visitor(&self, out: &mut Output) {
        let root = &self.krate.root;
        self.visit_module(root, out);
    }

    fn visit_module(&self, id: &Id, out: &mut Output) {
        let (_, module, name) = self.expect_module(id);
        out.segment_stack.push(name.to_owned());

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
            ItemEnum::Variant(_) => {}
            ItemEnum::Function(_) => self.visit_function(id, out),
            ItemEnum::Trait(_) => self.visit_trait(id, out),
            ItemEnum::TraitAlias(_) => self.visit_trait_alias(id, out), // ???
            ItemEnum::Impl(_) => todo!(),                               // ???
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
        let (_, union, _) = self.expect_union(id);
        self.visit_impls(&union.impls, out);
    }

    fn visit_struct(&self, id: &Id, out: &mut Output) {
        let (item, strukt, name) = self.expect_struct(id);
        let doc_string = Self::document_item(item, |out| {
            self.write_struct(out, strukt, name);
        });

        let path = out.to_path_string_with(name);
        out.index.push(path.into());
        out.docs.push(doc_string.into());

        out.segment_stack.push(name.to_owned());
        for imp in &strukt.impls {
            self.visit_impl(imp, out);
        }
        out.segment_stack.pop();
    }

    fn visit_enum(&self, id: &Id, out: &mut Output) {
        let (item, enu, name) = self.expect_enum(id);
        let doc_string = Self::document_item(item, |out| {
            self.write_enum(out, enu, name);
        });
        let path = out.to_path_string_with(name);
        out.index.push(path.into());
        out.docs.push(doc_string.into());
    }
    fn visit_function(&self, id: &Id, out: &mut Output) {
        let (item, function, name) = self.expect_function(id);
        let doc_string = Self::document_item(item, |out| {
            self.write_function(out, function, name);
        });
        let path = out.to_path_string_with(name);
        out.index.push(path.into());
        out.docs.push(doc_string.into());
    }
    fn visit_trait(&self, id: &Id, out: &mut Output) {}
    fn visit_trait_alias(&self, id: &Id, out: &mut Output) {}
    fn visit_typedef(&self, id: &Id, out: &mut Output) {}
    fn visit_macro(&self, id: &Id, out: &mut Output) {}
    fn visit_proc_macro(&self, id: &Id, out: &mut Output) {}
    fn visit_import(&self, id: &Id, out: &mut Output) {
        let (_, import) = self.expect_import(id);
        if let Some(id) = &import.id {
            // Some imports are not part of the same crate or not even resolved at all
            // Let's check if it exists in the index before visiting to avoid a panic
            if self.krate.index.contains_key(id) {
                self.visit_item(id, out);
            }
        }
    }

    fn visit_impl(&self, id: &Id, out: &mut Output) {
        let (_, imp) = self.expect_impl(id);
        for item in &imp.items {
            self.visit_item(item, out);
        }
    }
    fn visit_impls(&self, ids: &[Id], out: &mut Output) {
        for imp in ids {
            self.visit_impl(imp, out);
        }
    }

    fn document_item<F: FnOnce(&mut String)>(item: &Item, f: F) -> String {
        let mut out = String::from("```rs\n");
        f(&mut out);
        out.push_str("\n```\n");

        if let Some(docs) = &item.docs {
            if let Some(first_heading) = docs.find('#')
                && first_heading > 0 && first_heading < 300
            {
                out.push_str(&docs[..first_heading]);
            } else {
                out.extend(docs.chars().take(300));
            }

            if docs.len() > 300 {
                out.push_str("…\n");
            }
        }

        out
    }

    fn write_generics(&self, out: &mut String, generics: &Generics) {
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
    }

    fn write_impls(&self, out: &mut String, impls: &[Id], item_name: &str) {
        const MAX_INHERENT_IMPL_ITEMS: usize = 10;
        const MAX_TRAIT_IMPLS: usize = 10;
        out.push_str("impl ");
        out.push_str(item_name);
        out.push_str(" {\n");

        let items = impls.iter().map(|imp| self.expect_impl(imp));

        let total_items = items.clone().map(|(_, imp)| imp.items.len()).sum::<usize>();

        let inherent_impls = items
            .clone()
            .filter_map(|(_, imp)| {
                if imp.trait_.is_some() {
                    None
                } else {
                    Some(imp)
                }
            })
            .flat_map(|imp| imp.items.iter())
            .take(MAX_INHERENT_IMPL_ITEMS);

        for id in inherent_impls {
            let item = &self.krate.index[id];
            match &item.inner {
                ItemEnum::Function(f) => {
                    out.push_str("  ");
                    self.write_function(out, f, item.name.as_deref().unwrap());
                    out.push_str(";\n");
                }
                ItemEnum::AssocConst { type_, default } => {
                    out.push_str("const ");
                    out.push_str(item.name.as_deref().unwrap());
                    out.push_str(": ");
                    self.write_type(out, type_);
                    if let Some(default) = default {
                        out.push_str(" = ");
                        out.push_str(default);
                    }
                }
                other => println!("??: {other:?}"),
            }
        }

        if total_items > MAX_INHERENT_IMPL_ITEMS {
            let _ = writeln!(
                out,
                "  // {} more items",
                total_items - MAX_INHERENT_IMPL_ITEMS
            );
        }

        out.push('}');

        fn is_auto_trait(p: &Path) -> bool {
            ["RefUnwindSafe", "Send", "Sync", "Unpin", "UnwindSafe"].contains(&p.name.as_str())
        }

        fn is_blanket_trait_impl(p: &Path) -> bool {
            [
                "Any",
                "Borrow",
                "BorrowMut",
                "From",
                "Into",
                "ToOwned",
                "TryFrom",
                "TryInto",
            ]
            .contains(&p.name.as_str())
        }

        let trait_impls = items
            .clone()
            .filter_map(|(_, imp)| imp.trait_.as_ref().map(|t| (imp, t)))
            .filter(|(_, tr)| !is_blanket_trait_impl(tr))
            .sorted_by(|_, (_, tr2)| {
                if is_auto_trait(tr2) {
                    // Prefer inherent impls over auto trait impls
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
            .take(MAX_TRAIT_IMPLS);

        for (_, trait_) in trait_impls {
            out.push_str("\nimpl ");
            out.push_str(&trait_.name);
            out.push_str(" for ");
            out.push_str(item_name);
            out.push_str(" {} ");

            let hint = match trait_.name.as_str() {
                "Try" => Some("// the `?` operator"),
                "PartialEq" => Some("// the `==` operator"),
                "PartialOrd" => Some("// the `<`, `<=`, `>` and `>=` operators"),
                "Add" => Some("// the `+` operator"),
                "AddAssign" => Some("// the `+=` operator"),
                "BitAnd" => Some("// the `&` operator"),
                "BitAndAssign" => Some("// the `&=` operator"),
                "BitOr" => Some("// the `|` operator"),
                "BitOrAssign" => Some("// the `|=` operator"),
                "BitXor" => Some("// the `^` operator"),
                "BitXorAssign" => Some("// the `^=` operator"),
                "Deref" => Some("// code that is run on dereference (`*`) "),
                "DerefMut" => Some("// code that is run on mut dereference (`*`) "),
                "Div" => Some("// the `/` operator"),
                "DivAssign" => Some("// the `/=` operator"),
                "Drop" => Some("// code that is run when type is dropped"),
                "Fn" => Some("// the call operator that takes immutable env"),
                "FnMut" => Some("// the call operator that takes mutable env"),
                "FnOnce" => Some("// the call operator that takes by-value env"),
                "Index" => Some("// the indexing operator `[]` in immutable contexts"),
                "IndexMut" => Some("// the indexing operator `[]` in mutable contexts"),
                "Mul" => Some("// the `*` operator for multiplication"),
                "MulAssign" => Some("// the `*=` operator for multiplication"),
                "Neg" => Some("// the unary negation operator `-`"),
                "Not" => Some("// the unary logical negation operator `!`"),
                "Rem" => Some("// the remainder operator `%`"),
                "RemAssign" => Some("// the remainder assignment operator `%=`"),
                "Shl" => Some("// the left shift operator `<<`"),
                "ShlAssign" => Some("// the left shift assignment operator `<<=`"),
                "Shr" => Some("// the right shift operator `>>`"),
                "ShrAssign" => Some("// the right shift assignment operator `>>=`"),
                "Sub" => Some("// the `-` operator for subtraction"),
                "SubAssign" => Some("// the `-=` operator for subtraction"),
                _ => None,
            };

            if let Some(hint) = hint {
                out.push_str(hint);
            }
        }
    }

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
        self.write_generics(out, generics);

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
                    out.push_str("  // private fields omitted\n");
                }

                out.push_str("}\n");
                self.write_impls(out, impls, name);
            }
            StructKind::Unit => out.push(';'),
            StructKind::Tuple(fields) => {
                out.push('(');

                for (index, field) in fields.iter().enumerate() {
                    if index != 0 {
                        out.push_str(", ");
                    }

                    match field {
                        Some(field) => {
                            let (_, ty, _) = self.expect_struct_field(field);
                            self.write_type(out, ty);
                        }
                        None => out.push('_'),
                    }
                }

                out.push(')');
            }
        }
    }

    fn write_enum(
        &self,
        out: &mut String,
        Enum {
            generics,
            variants_stripped: _,
            variants,
            impls,
        }: &Enum,
        name: &str,
    ) {
        out.push_str("enum ");
        out.push_str(name);
        self.write_generics(out, generics);
        out.push_str(" {\n");
        for variant in variants {
            out.push_str("  ");
            let (_, variant, name) = self.expect_variant(variant);
            out.push_str(name);
            match &variant.kind {
                VariantKind::Plain => {}
                VariantKind::Tuple(tup) => {
                    out.push('(');
                    for (i, ty) in tup.iter().enumerate() {
                        if i != 0 {
                            out.push_str(", ");
                        }
                        match ty {
                            Some(x) => {
                                let (_, ty) = self.expect_unnamed_struct_field(x);
                                self.write_type(out, ty);
                            }
                            None => out.push('_'),
                        }
                    }
                    out.push(')');
                }
                VariantKind::Struct {
                    fields,
                    fields_stripped: _,
                } => {
                    out.push_str(" {\n");
                    for field in fields {
                        out.push_str("    ");
                        let (_, ty, name) = self.expect_struct_field(field);
                        out.push_str(name);
                        out.push_str(": ");
                        self.write_type(out, ty);
                        out.push_str(",\n");
                    }
                    out.push_str("  },\n");
                }
            }
            out.push_str(",\n");
        }
        out.push_str("}\n");
        self.write_impls(out, impls, name);
    }

    fn write_function_decl(&self, out: &mut String, decl: &FnDecl) {
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

    fn write_function_header(&self, out: &mut String, header: &Header) {
        if header.async_ {
            out.push_str("async ");
        }
        if header.const_ {
            out.push_str("const ");
        }
        if header.unsafe_ {
            out.push_str("unsafe ");
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
        self.write_function_header(out, header);
        out.push_str("fn ");
        out.push_str(name);
        self.write_generics(out, generics);

        out.push('(');
        self.write_function_decl(out, decl);
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
                                            out.push('\'');
                                            out.push_str(lt);
                                        }
                                        GenericArg::Infer => out.push('_'),
                                        GenericArg::Type(ty) => self.write_type(out, ty),
                                        GenericArg::Const(c) => {
                                            out.push_str("const ");
                                            out.push_str(&c.expr);
                                            out.push_str(": ");
                                            self.write_type(out, &c.type_);
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
            Type::QualifiedPath {
                name,
                args,
                self_type,
                trait_,
            } => {
                out.push_str(name); // TODO: not really correct
            }
            Type::ImplTrait(bounds) => {
                out.push_str("impl ");
                for (index, bound) in bounds.iter().enumerate() {
                    if index != 0 {
                        out.push_str(" + ");
                    }
                    match bound {
                        GenericBound::Outlives(lt) => {
                            out.push('\'');
                            out.push_str(lt);
                        }
                        GenericBound::TraitBound {
                            trait_,
                            generic_params,
                            modifier,
                        } => {
                            // TODO: dedupliate Path writing logic
                            out.push_str(&trait_.name);
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
