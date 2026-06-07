use qwert_core::CoreError;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    General = 1,
    Usage = 2,
    NotFound = 3,
    Conflict = 4,
    Validation = 5,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }

    pub fn category_str(self) -> &'static str {
        match self {
            ExitCode::Success => "success",
            ExitCode::General => "general",
            ExitCode::Usage => "usage",
            ExitCode::NotFound => "not_found",
            ExitCode::Conflict => "conflict",
            ExitCode::Validation => "validation",
        }
    }
}

impl From<&CoreError> for ExitCode {
    fn from(err: &CoreError) -> Self {
        match err {
            CoreError::NotFound(_) => ExitCode::NotFound,
            CoreError::PathTraversal(_)
            | CoreError::InvalidPattern(_)
            | CoreError::AppearanceValidation(_)
            | CoreError::InvalidUtf8 { .. } => ExitCode::Validation,
            CoreError::AppearanceConflict(_) => ExitCode::Conflict,
            CoreError::Toml(_) | CoreError::TomlSer(_) => ExitCode::Validation,
            CoreError::Json(_) => ExitCode::General,
            // AlreadyExists (rename target exists) → Conflict
            CoreError::Io(e) if e.kind() == std::io::ErrorKind::AlreadyExists => ExitCode::Conflict,
            CoreError::Io(_) => ExitCode::General,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_spec() {
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::General as i32, 1);
        assert_eq!(ExitCode::Usage as i32, 2);
        assert_eq!(ExitCode::NotFound as i32, 3);
        assert_eq!(ExitCode::Conflict as i32, 4);
        assert_eq!(ExitCode::Validation as i32, 5);
    }

    #[test]
    fn not_found_maps_to_3() {
        let e = CoreError::NotFound("missing.md".into());
        assert_eq!(ExitCode::from(&e), ExitCode::NotFound);
        assert_eq!(ExitCode::from(&e).as_i32(), 3);
    }

    #[test]
    fn traversal_maps_to_5() {
        let e = CoreError::PathTraversal("../secret".into());
        assert_eq!(ExitCode::from(&e), ExitCode::Validation);
    }

    #[test]
    fn conflict_maps_to_4() {
        let e = CoreError::AppearanceConflict("x".into());
        assert_eq!(ExitCode::from(&e), ExitCode::Conflict);
    }

    #[test]
    fn io_maps_to_1() {
        let e = CoreError::Io(std::io::Error::other("io"));
        assert_eq!(ExitCode::from(&e), ExitCode::General);
    }

    #[test]
    fn category_str_matches_spec() {
        assert_eq!(ExitCode::NotFound.category_str(), "not_found");
        assert_eq!(ExitCode::Validation.category_str(), "validation");
        assert_eq!(ExitCode::Conflict.category_str(), "conflict");
    }

    #[test]
    fn invalid_utf8_maps_to_5() {
        let e = CoreError::InvalidUtf8 { byte_offset: 42 };
        assert_eq!(ExitCode::from(&e), ExitCode::Validation);
        assert_eq!(ExitCode::from(&e).as_i32(), 5);
    }
}
