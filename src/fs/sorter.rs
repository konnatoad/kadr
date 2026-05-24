use crate::media::MediaEntry;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortMode {
    Name,
    NameReverse,
    Size,
    SizeReverse,
    Modified,
    ModifiedReverse,
    Type,
    Random,
}

impl SortMode {
    pub fn label(&self) -> &'static str {
        match self {
            SortMode::Name => "Name (A→Z)",
            SortMode::NameReverse => "Name (Z→A)",
            SortMode::Size => "Size (small→large)",
            SortMode::SizeReverse => "Size (large→small)",
            SortMode::Modified => "Date (oldest first)",
            SortMode::ModifiedReverse => "Date (newest first)",
            SortMode::Type => "Type",
            SortMode::Random => "Random",
        }
    }

    pub fn all() -> &'static [SortMode] {
        &[
            SortMode::Name,
            SortMode::NameReverse,
            SortMode::Size,
            SortMode::SizeReverse,
            SortMode::Modified,
            SortMode::ModifiedReverse,
            SortMode::Type,
            SortMode::Random,
        ]
    }
}

pub fn sort_entries(entries: &mut Vec<MediaEntry>, mode: &SortMode) {
    match mode {
        SortMode::Name => {
            entries.sort_by(|a, b| {
                natural_cmp(&a.file_name.to_lowercase(), &b.file_name.to_lowercase())
            });
        }
        SortMode::NameReverse => {
            entries.sort_by(|a, b| {
                natural_cmp(&b.file_name.to_lowercase(), &a.file_name.to_lowercase())
            });
        }
        SortMode::Size => {
            entries.sort_by_key(|e| e.file_size);
        }
        SortMode::SizeReverse => {
            entries.sort_by(|a, b| b.file_size.cmp(&a.file_size));
        }
        SortMode::Modified => {
            entries.sort_by(|a, b| a.modified.cmp(&b.modified));
        }
        SortMode::ModifiedReverse => {
            entries.sort_by(|a, b| b.modified.cmp(&a.modified));
        }
        SortMode::Type => {
            entries.sort_by(|a, b| {
                let ext_a = a.path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                let ext_b = b.path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                ext_a.cmp(&ext_b).then_with(|| natural_cmp(&a.file_name, &b.file_name))
            });
        }
        SortMode::Random => {
            entries.shuffle(&mut rand::rng());
        }
    }
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();

    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Less,
            (_, None) => return std::cmp::Ordering::Greater,
            (Some(ac), Some(bc)) if ac.is_ascii_digit() && bc.is_ascii_digit() => {
                let an: u64 = ai.by_ref().take_while(|c| c.is_ascii_digit())
                    .collect::<String>().parse().unwrap_or(0);
                let bn: u64 = bi.by_ref().take_while(|c| c.is_ascii_digit())
                    .collect::<String>().parse().unwrap_or(0);
                let ord = an.cmp(&bn);
                if ord != std::cmp::Ordering::Equal { return ord; }
            }
            (Some(ac), Some(bc)) => {
                let ord = ac.cmp(&bc);
                if ord != std::cmp::Ordering::Equal { return ord; }
                ai.next();
                bi.next();
            }
        }
    }
}
