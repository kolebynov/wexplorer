use std::borrow::Cow;

use url::Url;

pub trait UrlFilter {
    fn is_match(&self, url: &Url) -> bool;
}

#[derive(Clone)]
pub struct AllowedSchemeUrlFilter {
    allowed_schemes: Vec<String>,
}

impl AllowedSchemeUrlFilter {
    pub fn new(allowed_schemes: Vec<String>) -> Self {
        Self { allowed_schemes }
    }
}

impl UrlFilter for AllowedSchemeUrlFilter {
    fn is_match(&self, url: &Url) -> bool {
        self.allowed_schemes.iter().any(|s| url.scheme().eq_ignore_ascii_case(s))
    }
}

pub trait UrlNormalizer {
    fn normalize(&self, url: Url) -> Url;
}

#[derive(Clone)]
pub struct NoopNormalizer {}

impl UrlNormalizer for NoopNormalizer {
    fn normalize(&self, url: Url) -> Url {
        url
    }
}

pub struct UrlNormalizerBuilder<T = NoopNormalizer> {
    normalizer: T,
}

impl UrlNormalizerBuilder {
    pub fn new() -> Self {
        Self { normalizer: NoopNormalizer {} }
    }
}

impl<T: UrlNormalizer> UrlNormalizerBuilder<T> {
    pub fn add_normalizer<N: UrlNormalizer>(self, new_normalizer: N) -> UrlNormalizerBuilder<(N, T)> {
        UrlNormalizerBuilder { normalizer: (new_normalizer, self.normalizer) }
    }

    pub fn build(self) -> T {
        self.normalizer
    }
}

impl<I, O> UrlNormalizer for (I, O)
where
    I: UrlNormalizer,
    O: UrlNormalizer,
{
    fn normalize(&self, url: Url) -> Url {
        self.0.normalize(self.1.normalize(url))
    }
}

#[derive(Clone)]
pub struct RemoveFragmentNormalizer {}

impl UrlNormalizer for RemoveFragmentNormalizer {
    fn normalize(&self, mut url: Url) -> Url {
        url.set_fragment(None);
        url
    }
}

#[derive(Clone)]
pub enum QueryParamMatchType {
    Equals,
    StartWith,
}

#[derive(Clone)]
pub struct RemoveQueryParam(pub QueryParamMatchType, pub String);

#[derive(Clone)]
pub struct RemoveQueryParamsNormalizer {
    remove_query_params: Vec<RemoveQueryParam>,
}

impl RemoveQueryParamsNormalizer {
    pub fn new(remove_query_params: Vec<RemoveQueryParam>) -> Self {
        Self { remove_query_params }
    }
}

impl UrlNormalizer for RemoveQueryParamsNormalizer {
    fn normalize(&self, mut url: Url) -> Url {
        if url.query().is_none() {
            return url;
        }

        let mut found_matching_params = false;

        let remaining_query_params = url.query_pairs()
            .filter(|q| {
                let found = self.remove_query_params.iter()
                    .all(|r| match r.0 {
                        QueryParamMatchType::Equals => q.0.ne(&r.1),
                        QueryParamMatchType::StartWith => !q.0.starts_with(&r.1),
                    });
                found_matching_params |= found;
                found
            })
            .collect::<Vec<_>>();

        if remaining_query_params.is_empty() {
            url.set_query(None);
            return url;
        }

        if !found_matching_params {
            return url;
        }

        url.set_query(Some(&create_query_string(remaining_query_params.iter())));

        url
    }
}

#[derive(Clone)]
pub struct SortQueryParamsNormalizer {}

impl UrlNormalizer for SortQueryParamsNormalizer {
    fn normalize(&self, mut url: Url) -> Url {
        let query = url.query();
        if query.is_none() || query.unwrap() == "" {
            return url;
        }

        let mut query_params = url.query_pairs().collect::<Vec<_>>();
        query_params.sort_by(|q1, q2| q1.0.cmp(&q2.0));

        url.set_query(Some(&create_query_string(query_params.iter())));
        url
    }
}

fn create_query_string<'a>(params: impl Iterator<Item = &'a (Cow<'a, str>, Cow<'a, str>)>) -> String {
    let mut new_query = params
        .fold(String::new(), |mut query, (name, value)| {
            for encoded in form_urlencoded::byte_serialize(name.as_bytes()) {
                query.push_str(encoded);
            }

            if !value.is_empty() {
                query.push('=');
                for encoded in form_urlencoded::byte_serialize(value.as_bytes()) {
                    query.push_str(encoded);
                }
            }

            query.push('&');

            query
        });
    new_query.remove(new_query.len() - 1);
    new_query
}

#[derive(Clone)]
pub struct SchemeToLowerCaseNormalizer {}

impl UrlNormalizer for SchemeToLowerCaseNormalizer {
    fn normalize(&self, mut url: Url) -> Url {
        let scheme = url.scheme();
        if scheme.is_empty() || scheme.chars().all(|c| c.is_ascii_lowercase()) {
            return url;
        }

        let _ = url.set_scheme(&scheme.to_ascii_lowercase());
        url
    }
}

pub trait UrlProcessor {
    fn parse_url(&self, base_url: &Url, new_url: &str) -> Option<Url>;

    fn process_url(&self, url: Url) -> Option<Url>;
}

#[derive(Clone)]
pub struct UrlProcessorImpl<F, N> {
    filter: F,
    normalizer: N,
}

impl<F, N> UrlProcessorImpl<F, N>
where
    F: UrlFilter,
    N: UrlNormalizer,
{
    pub fn new(filter: F, normalizer: N) -> Self {
        Self { filter, normalizer }
    }
}

impl<F, N> UrlProcessor for UrlProcessorImpl<F, N>
where
    F: UrlFilter,
    N: UrlNormalizer,
{
    fn parse_url(&self, base_url: &Url, new_url: &str) -> Option<Url> {
        let parsed_url = base_url.join(new_url).ok()?;
        self.process_url(parsed_url)
    }

    fn process_url(&self, url: Url) -> Option<Url> {
        if !self.filter.is_match(&url) {
            return None;
        }

        Some(self.normalizer.normalize(url))
    }
}

#[cfg(test)]
mod remove_fragment_normalizer_tests {
    use super::*;

    #[test]
    fn should_remove_fragment() {
        // Arrange

        let expected = Url::parse("https://localhost/path?query").unwrap();
        let target = RemoveFragmentNormalizer {};

        // Act

        let result1 = target.normalize(Url::parse("https://localhost/path?query#fragment").unwrap());
        let result2 = target.normalize(Url::parse("https://localhost/path?query").unwrap());

        // Assert

        assert_eq!(expected, result1);
        assert_eq!(expected, result2);
    }
}

#[cfg(test)]
mod remove_query_params_normalizer_tests {
    use super::*;

    #[test]
    fn should_normalize_url_without_query() {
        // Arrange

        let expected = Url::parse("https://localhost/path").unwrap();
        let target = RemoveQueryParamsNormalizer::new(
            vec![RemoveQueryParam(QueryParamMatchType::Equals, "param".to_string())]);

        // Act

        let result1 = target.normalize(Url::parse("https://localhost/path").unwrap());
        let result2 = target.normalize(Url::parse("https://localhost/path?").unwrap());

        // Assert

        assert_eq!(expected, result1);
        assert_eq!(expected, result2);
    }

    #[test]
    fn should_remove_matched_query_params() {
        // Arrange

        let expected = Url::parse("https://localhost/path?param=value&param2=value2").unwrap();
        let target = RemoveQueryParamsNormalizer::new(
            vec![
                RemoveQueryParam(QueryParamMatchType::Equals, "param1".to_string()),
                RemoveQueryParam(QueryParamMatchType::StartWith, "utm_".to_string()),
            ]);

        // Act

        let result = target.normalize(Url::parse("https://localhost/path?param=value&param1&utm_v=value&param2=value2").unwrap());

        // Assert

        assert_eq!(expected, result);
    }

    #[test]
    fn should_remove_whole_query_if_all_params_matched() {
        // Arrange

        let expected = Url::parse("https://localhost/path").unwrap();
        let target = RemoveQueryParamsNormalizer::new(
            vec![
                RemoveQueryParam(QueryParamMatchType::Equals, "param1".to_string()),
                RemoveQueryParam(QueryParamMatchType::StartWith, "utm_".to_string()),
            ]);

        // Act

        let result = target.normalize(Url::parse("https://localhost/path?param1&utm_v=value").unwrap());

        // Assert

        assert_eq!(expected, result);
    }
}

#[cfg(test)]
mod sort_query_params_normalizer_tests {
    use super::*;

    #[test]
    fn should_untouch_url_without_query() {
        // Arrange

        let target = SortQueryParamsNormalizer {};

        // Act

        let result1 = target.normalize(Url::parse("https://localhost/path").unwrap());
        let result2 = target.normalize(Url::parse("https://localhost/path?").unwrap());

        // Assert

        assert_eq!(Url::parse("https://localhost/path").unwrap(), result1);
        assert_eq!(Url::parse("https://localhost/path?").unwrap(), result2);
    }

    #[test]
    fn should_sort_query_params() {
        // Arrange

        let target = SortQueryParamsNormalizer {};

        // Act

        let result = target.normalize(Url::parse("https://localhost/path?z=1&y=2&a").unwrap());

        // Assert

        assert_eq!(Url::parse("https://localhost/path?a&y=2&z=1").unwrap(), result);
    }
}