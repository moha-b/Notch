use super::Config;
use std::collections::HashSet;

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_appearance()?;
        self.validate_providers()?;
        if self
            .gemini_monthly_budget
            .is_some_and(|budget| budget == 0 || budget > 9_007_199_254_740_991)
        {
            return Err("Gemini budget must be a positive whole token count, or blank.".into());
        }
        if self.peek_seconds > 60 {
            return Err("Peek duration must be at most 60 seconds.".into());
        }
        if self.port == 0 {
            return Err("Hook port must be between 1 and 65535.".into());
        }
        if !["auto", "en", "zh", "ja", "ko"].contains(&self.lang.as_str()) {
            return Err("Unknown interface language.".into());
        }
        Ok(())
    }

    fn validate_placement(&self) -> Result<(), String> {
        if !["left", "right", "top", "bottom"].contains(&self.edge.as_str()) {
            return Err("Unknown screen edge.".into());
        }
        if !self.scale.is_finite() || !(0.5..=2.0).contains(&self.scale) {
            return Err("Size must be between 50% and 200%.".into());
        }
        if !self.notch_y.is_finite() || !(0.0..=1.0).contains(&self.notch_y) {
            return Err("Offset must be between 0 and 1.".into());
        }
        Ok(())
    }

    fn validate_appearance(&self) -> Result<(), String> {
        self.validate_placement()?;
        if self.accent.len() != 7
            || !self.accent.starts_with('#')
            || !self.accent[1..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("Accent must be a hex color.".into());
        }
        if !["relative", "absolute"].contains(&self.reset_format.as_str()) {
            return Err("Unknown reset time format.".into());
        }
        Ok(())
    }

    fn validate_providers(&self) -> Result<(), String> {
        let order: HashSet<_> = self.provider_order.iter().map(String::as_str).collect();
        if order.len() != self.provider_order.len()
            || crate::providers::FAMILIES
                .iter()
                .any(|id| !order.contains(id))
        {
            return Err("Provider order must contain each family once, without duplicates.".into());
        }
        if self
            .provider_order
            .iter()
            .chain(&self.disabled_providers)
            .chain(&self.muted_providers)
            .any(|id| !valid_provider_id(id))
        {
            return Err("Unknown provider identifier.".into());
        }
        Ok(())
    }
}

fn valid_provider_id(id: &str) -> bool {
    if crate::providers::FAMILIES.contains(&id) {
        return true;
    }
    let slug = id
        .strip_prefix("claude-")
        .or_else(|| id.strip_prefix("codex-"));
    slug.is_some_and(|slug| {
        !slug.is_empty()
            && !slug
                .chars()
                .any(|character| character.is_control() || matches!(character, '/' | '\\'))
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn invalid_saved_values_are_rejected_before_window_placement() {
        for change in [
            json!({"scale":0}),
            json!({"notch_y":2}),
            json!({"edge":"diagonal"}),
            json!({"accent":"red"}),
            json!({"reset_format":"bad"}),
            json!({"peek_seconds":61}),
            json!({"port":0}),
            json!({"gemini_monthly_budget":0}),
            json!({"gemini_monthly_budget":9007199254740992u64}),
            json!({"provider_order":["claude","claude"]}),
        ] {
            assert!(
                super::super::decode(change.to_string().as_bytes()).is_err(),
                "{change}"
            );
        }
        assert!(super::super::decode(br#"{"scale":0.5,"notch_y":0}"#).is_ok());
        assert!(super::super::decode(br#"{"gemini_monthly_budget":null}"#).is_ok());
        assert!(super::super::decode(br#"{"gemini_monthly_budget":1000000}"#).is_ok());
        assert!(super::super::decode(br#"{"scale":2,"notch_y":1,"peek_seconds":60}"#).is_ok());
    }

    #[test]
    fn corrupt_settings_remain_available_for_recovery() {
        let directory = std::env::temp_dir().join(format!(
            "notch-settings-{}-{}",
            std::process::id(),
            crate::providers::now_ms()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("config.json");
        let original = br#"{"scale":-1,"disabled_providers":["claude"]}"#;
        std::fs::write(&path, original).unwrap();
        let recovered = super::super::load_from(&path).unwrap();
        assert_eq!(recovered.scale, 1.0);
        assert_eq!(
            recovered.disabled_providers.len(),
            crate::providers::FAMILIES.len()
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let backup = std::fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|candidate| candidate != &path)
            .unwrap();
        assert_eq!(std::fs::read(backup).unwrap(), original);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
