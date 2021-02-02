//! Code relating to the generation of the https://rustsec.org web site.
//!
use crate::prelude::*;
use std::{fs, path::PathBuf};

use askama::Template;
use comrak::{markdown_to_html, ComrakOptions};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

#[derive(Template)]
#[template(path = "advisories.html")]
struct AdvisoriesListTemplate {
    advisories_per_year: Vec<AdvisoriesPerYear>,
}

struct AdvisoriesPerYear {
    year: u32,
    advisories: Vec<rustsec::Advisory>,
}

#[derive(Template)]
#[template(path = "advisory.html")]
struct AdvisoryTemplate<'a> {
    advisory: &'a rustsec::Advisory,
    rendered_description: String,
}

/// Render all advisories using the Markdown template
pub fn render_advisories() {
    let mut advisories: Vec<rustsec::Advisory> = rustsec::Database::fetch()
        .unwrap()
        .iter()
        .map(|advisory| advisory.to_owned())
        .collect();

    let output_folder = PathBuf::from("_site");

    // Render individual advisory pages.
    let advisories_folder = output_folder.join("advisories");
    fs::create_dir_all(&advisories_folder).unwrap();

    for advisory in &advisories {
        let output_path = advisories_folder.join(advisory.id().as_str().to_owned() + ".html");

        let rendered_description =
            markdown_to_html(advisory.description(), &ComrakOptions::default());

        let advisory_tmpl = AdvisoryTemplate {
            advisory,
            rendered_description,
        };
        fs::write(&output_path, advisory_tmpl.render().unwrap()).unwrap();

        status_ok!("Rendered", "{}", output_path.display());
    }

    // Copy all the static assets.
    let mut options = fs_extra::dir::CopyOptions::new();
    options.overwrite = true;
    options.content_only = true;

    let static_folder = PathBuf::from("src/web/static");
    fs_extra::dir::copy(static_folder, &output_folder, &options).unwrap();

    // Render the index.html (/) page.
    let index_page = IndexTemplate.render().unwrap();
    fs::write(output_folder.join("index.html"), index_page).unwrap();

    // Render the advisories.html (/advisories) page.

    // Sort the advisories by date in descending order for the big listing.
    advisories.sort_by(|a, b| b.date().cmp(&a.date()));

    let mut advisories_per_year = Vec::<AdvisoriesPerYear>::new();
    for advisory in advisories.clone() {
        // If we have an AdvisoriesPerYear for this advisory's year, just
        // push it into that list. Otherwise, set one up for the year.
        match advisories_per_year
            .iter_mut()
            .find(|per_year| per_year.year == advisory.date().year())
        {
            Some(advisories_for_year) => advisories_for_year.advisories.push(advisory),
            None => advisories_per_year.push(AdvisoriesPerYear {
                year: advisory.date().year(),
                advisories: vec![advisory],
            }),
        }
    }

    let advisories_page_tmpl = AdvisoriesListTemplate {
        advisories_per_year,
    };
    let advisories_page = advisories_page_tmpl.render().unwrap();
    fs::write(advisories_folder.join("index.html"), advisories_page).unwrap();

    status_ok!(
        "Completed",
        "{} advisories rendered as HTML",
        advisories.len()
    );
}

mod filters {
    use chrono::NaiveDate;
    use std::convert::TryInto;

    pub fn friendly_date(date: &&rustsec::advisory::Date) -> ::askama::Result<String> {
        Ok(
            NaiveDate::from_ymd(date.year().try_into().unwrap(), date.month(), date.day())
                .format("%B %e, %Y")
                .to_string(),
        )
    }
}
