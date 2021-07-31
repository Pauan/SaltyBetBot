#[derive(Clone, Debug)]
pub struct RegExp(js_sys::RegExp);

impl RegExp {
    #[inline]
    pub fn new(pattern: &str) -> Self {
        // TODO is the u flag correct ?
        Self::new_with_flags(pattern, "gu")
    }

    #[inline]
    pub fn new_with_flags(pattern: &str, flags: &str) -> Self {
        Self(js_sys::RegExp::new(pattern, flags))
    }

    #[inline]
    pub fn is_match(&self, input: &str) -> bool {
        let is_match = self.0.test(input);
        self.0.set_last_index(0);
        is_match
    }

    #[inline]
    fn exec(&self, input: &str) -> Option<Vec<Option<String>>> {
        let array = self.0.exec(input)?;
        Some(array.iter().map(|x| x.as_string()).collect())
    }

    #[inline]
    pub fn all_matches(&self, input: &str) -> Vec<Vec<Option<String>>> {
        let mut matches = vec![];

        loop {
            if let Some(array) = self.exec(input) {
                matches.push(array);

            } else {
                break;
            }
        }

        matches
    }

    #[inline]
    pub fn first_match(&self, input: &str) -> Option<Vec<Option<String>>> {
        let matches = self.exec(input);
        self.0.set_last_index(0);
        matches
    }

    #[inline]
    pub fn replace(&self, input: &str, replace: &str) -> String {
        js_sys::JsString::from(input).replace_by_pattern(&self.0, replace).into()
    }
}
