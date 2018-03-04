use stdweb::Reference;
use stdweb::unstable::TryInto;


// TODO move this into stdweb
#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "RegExp")]
pub struct RegExp(Reference);

impl RegExp {
    #[inline]
    pub fn new(pattern: &str) -> Self {
        // TODO is the u flag correct ?
        Self::new_with_flags(pattern, "gu")
    }

    #[inline]
    pub fn new_with_flags(pattern: &str, flags: &str) -> Self {
        js!( return new RegExp(@{pattern}, @{flags}); ).try_into().unwrap()
    }

    #[inline]
    pub fn is_match(&self, input: &str) -> bool {
        js!(
            var self = @{self};
            var is_match = self.test(@{input});
            self.lastIndex = 0;
            return is_match;
        ).try_into().unwrap()
    }

    #[inline]
    pub fn all_matches(&self, input: &str) -> Vec<Vec<Option<String>>> {
        js!(
            var self = @{self};
            var input = @{input};
            var matches = [];
            var array;

            while ((array = self.exec(input)) !== null) {
                matches.push(array);
            }

            return matches;
        ).try_into().unwrap()
    }

    #[inline]
    pub fn first_match(&self, input: &str) -> Option<Vec<Option<String>>> {
        js!(
            var self = @{self};
            var array = self.exec(@{input});
            self.lastIndex = 0;
            return array;
        ).try_into().unwrap()
    }

    #[inline]
    pub fn replace(&self, input: &str, replace: &str) -> String {
        js!(
            return @{input}.replace(@{self}, @{replace});
        ).try_into().unwrap()
    }
}
