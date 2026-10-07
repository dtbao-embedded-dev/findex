use crate::error::{AppError, AppResult};

/// Greets `name`, refusing a blank one.
pub fn greet(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("a name is required".into()));
    }
    Ok(format!("Hello, {name}!"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_a_trimmed_name() {
        assert_eq!(greet("  Ada ").unwrap(), "Hello, Ada!");
    }

    #[test]
    fn refuses_a_blank_name() {
        assert!(matches!(greet("   "), Err(AppError::Invalid(_))));
    }
}
