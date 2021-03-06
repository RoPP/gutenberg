#[macro_use]
extern crate serde_derive;
extern crate tera;
extern crate slug;

extern crate errors;
extern crate config;
extern crate content;
extern crate front_matter;

use std::collections::HashMap;

use slug::slugify;
use tera::{Context, Tera};

use config::Config;
use errors::{Result, ResultExt};
use content::{Page, sort_pages};
use front_matter::SortBy;


#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TaxonomyKind {
    Tags,
    Categories,
}

/// A tag or category
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TaxonomyItem {
    pub name: String,
    pub slug: String,
    pub pages: Vec<Page>,
}

impl TaxonomyItem {
    pub fn new(name: &str, pages: Vec<Page>) -> TaxonomyItem {
        // We shouldn't have any pages without dates there
        let (sorted_pages, _) = sort_pages(pages, SortBy::Date);

        TaxonomyItem {
            name: name.to_string(),
            slug: slugify(name),
            pages: sorted_pages,
        }
    }
}

/// All the tags or categories
#[derive(Debug, Clone, PartialEq)]
pub struct Taxonomy {
    pub kind: TaxonomyKind,
    // this vec is sorted by the count of item
    pub items: Vec<TaxonomyItem>,
}

impl Taxonomy {
    // TODO: take a Vec<&'a Page> if it makes a difference in terms of perf for actual sites
    pub fn find_tags_and_categories(all_pages: &[Page]) -> (Taxonomy, Taxonomy) {
        let mut tags = HashMap::new();
        let mut categories = HashMap::new();

        // Find all the tags/categories first
        for page in all_pages {
            // Don't consider pages without pages for tags/categories as that's the only thing
            // we can sort pages with across sections
            // If anyone sees that comment and wonder wtf, please open an issue as I can't think of
            // usecases other than blog posts for built-in taxonomies
            if page.meta.date.is_none() {
                continue;
            }

            if let Some(ref category) = page.meta.category {
                categories
                    .entry(category.to_string())
                    .or_insert_with(|| vec![])
                    .push(page.clone());
            }

            if let Some(ref t) = page.meta.tags {
                for tag in t {
                    tags
                        .entry(tag.to_string())
                        .or_insert_with(|| vec![])
                        .push(page.clone());
                }
            }
        }

        // Then make TaxonomyItem out of them, after sorting it
        let tags_taxonomy = Taxonomy::new(TaxonomyKind::Tags, tags);
        let categories_taxonomy = Taxonomy::new(TaxonomyKind::Categories, categories);

        (tags_taxonomy, categories_taxonomy)
    }

    fn new(kind: TaxonomyKind, items: HashMap<String, Vec<Page>>) -> Taxonomy {
        let mut sorted_items = vec![];
        for (name, pages) in &items {
            sorted_items.push(
                TaxonomyItem::new(name, pages.clone())
            );
        }
        sorted_items.sort_by(|a, b| b.pages.len().cmp(&a.pages.len()));

        Taxonomy {
            kind,
            items: sorted_items,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_single_item_name(&self) -> String {
        match self.kind {
            TaxonomyKind::Tags => "tag".to_string(),
            TaxonomyKind::Categories => "category".to_string(),
        }
    }

    pub fn get_list_name(&self) -> String {
        match self.kind {
            TaxonomyKind::Tags => "tags".to_string(),
            TaxonomyKind::Categories => "categories".to_string(),
        }
    }

    pub fn render_single_item(&self, item: &TaxonomyItem, tera: &Tera, config: &Config) -> Result<String> {
        let name = self.get_single_item_name();
        let mut context = Context::new();
        context.add("config", config);
        context.add(&name, item);
        context.add("current_url", &config.make_permalink(&format!("{}/{}", name, item.slug)));
        context.add("current_path", &format!("/{}/{}", name, item.slug));

        tera.render(&format!("{}.html", name), &context)
            .chain_err(|| format!("Failed to render {} page.", name))
    }

    pub fn render_list(&self, tera: &Tera, config: &Config) -> Result<String> {
        let name = self.get_list_name();
        let mut context = Context::new();
        context.add("config", config);
        context.add(&name, &self.items);
        context.add("current_url", &config.make_permalink(&name));
        context.add("current_path", &name);

        tera.render(&format!("{}.html", name), &context)
            .chain_err(|| format!("Failed to render {} page.", name))
    }
}
