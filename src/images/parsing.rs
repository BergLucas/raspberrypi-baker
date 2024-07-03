use chrono::NaiveDateTime;
use scraper::{ElementRef, Html};

pub struct ApacheFile {
    name: String,
    last_modified: NaiveDateTime,
    is_directory: bool,
}

impl ApacheFile {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn last_modified(&self) -> NaiveDateTime {
        self.last_modified
    }
    pub fn is_directory(&self) -> bool {
        self.is_directory
    }
}

fn handle_element(element: ElementRef) -> Result<Option<ApacheFile>, Box<dyn std::error::Error>> {
    let mut children = element.children().filter_map(ElementRef::wrap);

    if element.children().count() != 5 {
        return Ok(None);
    }

    let filetype = children
        .next()
        .ok_or("Missing file element")?
        .select(&scraper::Selector::parse("img").unwrap())
        .next()
        .ok_or("Missing img element")?
        .value()
        .attr("alt")
        .ok_or("Missing alt attribute")?;

    let is_directory = match filetype {
        "[DIR]" => true,
        "[   ]" => false,
        _ => return Ok(None),
    };

    let name = children
        .next()
        .ok_or("Missing name element")?
        .select(&scraper::Selector::parse("a").unwrap())
        .next()
        .ok_or("Missing href element")?
        .inner_html()
        .trim_end_matches("/")
        .to_string();

    let last_modified = NaiveDateTime::parse_from_str(
        children
            .next()
            .ok_or("Missing date element")?
            .inner_html()
            .trim(),
        "%Y-%m-%d %H:%M",
    )?;

    Ok(Some(ApacheFile {
        name,
        last_modified,
        is_directory,
    }))
}

pub fn parse_apache_directory_listing(
    body: &str,
) -> Result<Vec<ApacheFile>, Box<dyn std::error::Error>> {
    Html::parse_document(body)
        .select(&scraper::Selector::parse("tr").unwrap())
        .filter_map(|element| handle_element(element).transpose())
        .collect()
}
