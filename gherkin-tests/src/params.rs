use std::str::FromStr;

use cucumber::Parameter;

#[derive(Parameter, Debug, Hash, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[param(regex = r"[\w\d]+", name = "identifier")]
pub struct Identifier(String);

impl FromStr for Identifier {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Identifier(s.to_string()))
    }
}
