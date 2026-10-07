//! Reuses every existing native control without giving a peer a second player
//! authority. The underlying decoder is used only for the synchronized preview.
#[derive(Clone, Copy)]
pub struct Player(pub &'static libmpv2::Mpv);
impl std::ops::Deref for Player {
    type Target = libmpv2::Mpv;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
impl Player {
    pub fn command(&self, name: &str, args: &[&str]) -> libmpv2::Result<()> {
        if let Some(client) = crate::peer::client()
            && !crate::peer::mirroring()
        {
            if name == "osd-overlay" {
                return self.0.command(name, args);
            }
            let result = if name == "loadfile" {
                client.queue("/api/player/command",serde_json::json!({"command":"open","target":args.first().copied().unwrap_or("")}))
            } else {
                client.queue(
                    "/api/peer/media",
                    serde_json::json!({"operation":"command","name":name,"args":args}),
                )
            };
            if let Err(error) = result {
                log::warn!("{error}");
                return Err(libmpv2::Error::Null);
            }
            return Ok(());
        }
        self.0.command(name, args)
    }
    pub fn set_property<T: libmpv2::SetData + serde::Serialize>(
        &self,
        name: &str,
        value: T,
    ) -> libmpv2::Result<()> {
        if let Some(client) = crate::peer::client()
            && !crate::peer::mirroring()
        {
            // Subtitle shaping/render configuration belongs to this preview.
            // These do not represent user mutations of the media session.
            if matches!(
                name,
                "sub-font"
                    | "osd-font"
                    | "sub-fonts-dir"
                    | "sub-ass-override"
                    | "http-proxy"
                    | "network-timeout"
                    | "user-agent"
                    | "options/user-agent"
            ) {
                return self.0.set_property(name, value);
            }
            let value = serde_json::to_value(value).map_err(|_| libmpv2::Error::Null)?;
            client
                .queue(
                    "/api/peer/media",
                    serde_json::json!({"operation":"set_property","name":name,"value":value}),
                )
                .map_err(|error| {
                    log::warn!("{error}");
                    libmpv2::Error::Null
                })?;
            return Ok(());
        }
        self.0.set_property(name, value)
    }
}
