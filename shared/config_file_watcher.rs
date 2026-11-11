pub struct ConfigFileWatcher {
    _watcher: notify::RecommendedWatcher,
    events: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
}

impl ConfigFileWatcher {
    pub fn new(
        path: &std::path::Path,
        wake_ui: impl Fn() + Send + 'static,
    ) -> Result<Self, String> {
        use notify::Watcher;

        let parent = path
            .parent()
            .ok_or_else(|| format!("configuration path has no parent: {}", path.display()))?;
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "create configuration directory {}: {error}",
                parent.display()
            )
        })?;
        let (sender, events) = std::sync::mpsc::channel();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let _ = sender.send(event);
                wake_ui();
            })
            .map_err(|error| format!("create configuration file watcher: {error}"))?;
        watcher
            .watch(parent, notify::RecursiveMode::NonRecursive)
            .map_err(|error| {
                format!(
                    "watch configuration directory {}: {error}",
                    parent.display()
                )
            })?;
        Ok(Self {
            _watcher: watcher,
            events,
        })
    }

    pub fn take_changed(&self) -> Result<bool, String> {
        let mut changed = false;
        let mut last_error = None;
        while let Ok(event) = self.events.try_recv() {
            match event {
                Ok(event) if !matches!(event.kind, notify::EventKind::Access(_)) => changed = true,
                Ok(_) => {}
                Err(error) => last_error = Some(error.to_string()),
            }
        }
        last_error.map_or(Ok(changed), |error| {
            Err(format!("configuration file watcher: {error}"))
        })
    }
}
