use std::str::FromStr;

pub enum ParseError {
    InvalidCategory(String),
    // InvalidCategories(String)
}

#[derive(Debug)]
pub struct Category {
    pub text: String
}

#[derive(Debug)]
pub struct Categories {
    pub categories: Vec<Category>
}

impl FromStr for Category {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(ParseError::InvalidCategory("Category text is empty.".to_string()));
        }

        Ok(
            Self {
                text: value.to_string()
            }
        )
    }

}
