use itertools::Itertools;
use rustdoc_types::Crate;
use rustdoc_types::Id;
use rustdoc_types::Impl;
use rustdoc_types::Item;
use rustdoc_types::ItemEnum;
use rustdoc_types::Type;

use crate::util::fuzzy_match;

#[derive(Debug)]
pub struct Docs {
    crates: Vec<Crate>,
}

impl Docs {
    pub fn new() -> Self {
        Self { crates: Vec::new() }
    }

    pub fn crate_struct_field(&self, crate_id: usize, ty_id: &Id) -> Option<(&Item, &Type)> {
        self.crates[crate_id].index.get(ty_id).and_then(|i| {
            Some((
                i,
                match &i.inner {
                    ItemEnum::StructField(ty) => ty,
                    _ => return None,
                },
            ))
        })
    }

    pub fn crate_struct_impl(&self, crate_id: usize, impl_id: &Id) -> Option<(&Item, &Impl)> {
        self.crates[crate_id].index.get(impl_id).and_then(|i| {
            Some((
                i,
                match &i.inner {
                    ItemEnum::Impl(imp) => imp,
                    _ => return None,
                },
            ))
        })
    }

    pub fn crate_item(&self, crate_id: usize, item_id: &Id) -> Option<&Item> {
        self.crates[crate_id].index.get(item_id)
    }

    pub fn add_crate_json(&mut self, source: &str) -> Result<(), serde_json::Error> {
        let krate = serde_json::from_str(source)?;
        self.crates.push(krate);
        Ok(())
    }

    fn find_parent_item_in_crate(&self, id: usize, child_id: &Id) -> Option<&Item> {
        self.crates[id]
            .index
            .values()
            .find(|item| match &item.inner {
                ItemEnum::Impl(imp) => imp.items.contains(child_id),
                ItemEnum::Module(mo) => mo.items.contains(child_id),
                _ => false,
            })
    }

    fn find_in_crate<'a: 'b, 'b>(
        &'a self,
        id: usize,
        query: &'b [&'b str],
    ) -> impl Iterator<Item = (isize, &'a Item)> + 'b {
        self.crates[id]
            .index
            .iter()
            .filter_map(move |(item_id, item)| {
                if let [query] = query {
                    let score = fuzzy_match(query, item.name.as_deref()?)?;
                    Some((score, item))
                } else if let [parent_query, query] | [.., parent_query, query] = query {
                    let score = fuzzy_match(query, item.name.as_deref()?)?;
                    if score < 100 {
                        return None;
                    }

                    let parent = self.find_parent_item_in_crate(id, item_id)?;
                    let parent_name = match &parent.inner {
                        ItemEnum::Impl(Impl {
                            for_: Type::ResolvedPath(path),
                            ..
                        }) => &path.name,
                        ItemEnum::Module(_) => parent.name.as_deref().unwrap(),
                        _ => return None,
                    };
                    let score_parent = fuzzy_match(parent_query, parent_name)?;
                    Some((score_parent.saturating_add(score), item))
                } else {
                    None
                }
            })
    }

    /// Returns (item, crate_id)
    pub fn find<'a>(&'a self, query: &str) -> Option<(&'a Item, usize)> {
        let query = query.split("::").collect::<Vec<_>>();
        (0..self.crates.len())
            .flat_map(|id| {
                self.find_in_crate(id, query.as_slice())
                    .map(move |(score, item)| (score, item, id))
            })
            .sorted_by(|(a, ..), (b, ..)| b.cmp(a))
            .map(|(_, item, index)| (item, index))
            .next()
    }
}
