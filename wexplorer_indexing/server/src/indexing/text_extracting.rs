use ego_tree::iter::Edge;
use itertools::Itertools;
use scraper::{Html, Selector, ElementRef, Node};

#[derive(Clone)]
pub struct TextExtractor {
    body_selector: Selector,
}

impl TextExtractor {
    pub fn new() -> Self {
        Self { body_selector: Selector::parse("body").unwrap() }
    }

    pub fn extract_text(&self, html: &Html) -> Option<String> {
        html.select(&self.body_selector)
            .next()
            .map(|body| extract_text_from_body(body))
            .and_then(|text| if text.is_empty() { None } else { Some(text) })
    }
}

fn extract_text_from_body(body: ElementRef) -> String {
    let skip_elements = get_skip_elements();
    body.traverse()
        .filter_map(|edge| {
            let Edge::Open(node) = edge else { return None; };
            let Node::Text(text) = node.value() else { return None; };
            let text = Some(text.trim());
            let Some(parent) = node.parent() else { return text; };
            let Node::Element(parent_element) = parent.value() else { return text; };

            if skip_elements.contains(&parent_element.name()) {
                None
            }
            else {
                text
            }
        })
        .join(" ")
}

fn get_skip_elements() -> &'static [&'static str] {
    &["script", "style"]
}

#[cfg(test)]
mod text_extractor_tests {
    use super::*;
    use scraper::Html;

    #[test]
    fn should_extract_text_from_body() {
        // Arrange

        let html = Html::parse_document("<html><body>Test<div>text<span>example</span></div>text</body></html>");
        let target = TextExtractor::new();

        // Act

        let result = target.extract_text(&html).unwrap();

        // Assert

        assert_eq!("Test text example text", result);
    }

    #[test]
    fn should_not_extract_text_from_skip_elements() {
        // Arrange

        let html = Html::parse_document("<html><body>Test<SCRIPT>script</SCRIPT><Style>style</Style></body></html>");
        let target = TextExtractor::new();

        // Act

        let result = target.extract_text(&html).unwrap();

        // Assert

        assert_eq!("Test", result);
    }
}