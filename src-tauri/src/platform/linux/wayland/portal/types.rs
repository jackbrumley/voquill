#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalShortcutsFlow {
    RestoreExisting,
    BindNew,
}

impl GlobalShortcutsFlow {
    pub fn from_force(force: bool) -> Self {
        if force {
            Self::BindNew
        } else {
            Self::RestoreExisting
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RestoreExisting => "restore-existing",
            Self::BindNew => "bind-new",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GlobalShortcutsFlow;

    #[test]
    fn flow_from_force_maps_false_to_restore() {
        assert_eq!(
            GlobalShortcutsFlow::from_force(false),
            GlobalShortcutsFlow::RestoreExisting
        );
    }

    #[test]
    fn flow_from_force_maps_true_to_bind() {
        assert_eq!(
            GlobalShortcutsFlow::from_force(true),
            GlobalShortcutsFlow::BindNew
        );
    }
}
