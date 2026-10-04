use libmpv2::Mpv;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MediaPropertyGroup {
    pub name: String,
    pub properties: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MediaCollectionEntry {
    pub index: i64,
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaChapter {
    pub index: i64,
    pub title: String,
    pub time_seconds: f64,
}

/// A static, factual snapshot read directly from libmpv after a file loads or
/// when the user explicitly refreshes the Media Inspector. Exact libmpv keys
/// are retained so the diagnostic surface never invents or obscures data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MediaFileInfo {
    pub loaded: bool,
    pub groups: Vec<MediaPropertyGroup>,
    pub metadata: BTreeMap<String, String>,
    pub filtered_metadata: BTreeMap<String, String>,
    pub chapters: Vec<MediaCollectionEntry>,
    pub editions: Vec<MediaCollectionEntry>,
    pub playlist: Vec<MediaCollectionEntry>,
}

const PROPERTY_GROUPS: &[(&str, &[&str])] = &[
    (
        "File and source",
        &[
            "filename",
            "filename/no-ext",
            "path",
            "stream-open-filename",
            "media-title",
            "file-size",
            "file-format",
            "demuxer",
            "demuxer-via-network",
        ],
    ),
    (
        "Playback and selection",
        &[
            "duration",
            "time-pos",
            "percent-pos",
            "speed",
            "pause",
            "seekable",
            "partially-seekable",
            "eof-reached",
            "vid",
            "aid",
            "sid",
            "secondary-sid",
            "chapter",
            "chapters",
            "edition",
            "editions",
        ],
    ),
    (
        "Video",
        &[
            "video-format",
            "video-codec",
            "video-bitrate",
            "width",
            "height",
            "dwidth",
            "dheight",
            "video-aspect",
            "container-fps",
            "estimated-vf-fps",
            "video-params/pixelformat",
            "video-params/hw-pixelformat",
            "video-params/w",
            "video-params/h",
            "video-params/dw",
            "video-params/dh",
            "video-params/aspect",
            "video-params/par",
            "video-params/colormatrix",
            "video-params/colorlevels",
            "video-params/primaries",
            "video-params/gamma",
            "video-params/sig-peak",
            "video-params/light",
            "video-params/chroma-location",
            "video-params/rotate",
            "video-params/stereo-in",
            "video-params/avg-bpp",
            "video-params/alpha",
        ],
    ),
    (
        "Audio",
        &[
            "audio-format",
            "audio-codec-name",
            "audio-codec",
            "audio-bitrate",
            "audio-samplerate",
            "audio-channels",
            "audio-params/format",
            "audio-params/samplerate",
            "audio-params/channels",
            "audio-params/channel-count",
            "audio-params/hr-channels",
        ],
    ),
    (
        "Cache and network",
        &[
            "cache",
            "cache-pause",
            "cache-buffering-state",
            "demuxer-cache-duration",
            "demuxer-cache-time",
            "demuxer-cache-idle",
            "paused-for-cache",
        ],
    ),
];

fn property_text(mpv: &Mpv, name: &str) -> Option<String> {
    mpv.get_property::<String>(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            mpv.get_property::<i64>(name)
                .ok()
                .map(|value| value.to_string())
        })
        .or_else(|| {
            mpv.get_property::<f64>(name)
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| format!("{value:.6}"))
        })
        .or_else(|| {
            mpv.get_property::<bool>(name)
                .ok()
                .map(|value| if value { "yes" } else { "no" }.to_string())
        })
}

fn property_map(mpv: &Mpv, root: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    let count = mpv
        .get_property::<i64>(&format!("{root}/list/count"))
        .unwrap_or(0)
        .max(0);
    for index in 0..count {
        let prefix = format!("{root}/list/{index}");
        let Some(key) = property_text(mpv, &format!("{prefix}/key")) else {
            continue;
        };
        if let Some(value) = property_text(mpv, &format!("{prefix}/value")) {
            values.insert(key, value);
        }
    }
    values
}

fn collection(mpv: &Mpv, root: &str, properties: &[&str]) -> Vec<MediaCollectionEntry> {
    let count = mpv
        .get_property::<i64>(&format!("{root}/count"))
        .unwrap_or(0)
        .max(0);
    (0..count)
        .map(|index| {
            let prefix = format!("{root}/{index}");
            let properties = properties
                .iter()
                .filter_map(|name| {
                    property_text(mpv, &format!("{prefix}/{name}"))
                        .map(|value| ((*name).to_string(), value))
                })
                .collect();
            MediaCollectionEntry { index, properties }
        })
        .collect()
}

pub fn capture(mpv: &Mpv) -> MediaFileInfo {
    let mut groups = Vec::new();
    for (name, keys) in PROPERTY_GROUPS {
        let properties = keys
            .iter()
            .filter_map(|key| property_text(mpv, key).map(|value| ((*key).to_string(), value)))
            .collect::<Vec<_>>();
        if !properties.is_empty() {
            groups.push(MediaPropertyGroup {
                name: (*name).to_string(),
                properties,
            });
        }
    }

    MediaFileInfo {
        loaded: !groups.is_empty(),
        groups,
        metadata: property_map(mpv, "metadata"),
        filtered_metadata: property_map(mpv, "filtered-metadata"),
        chapters: collection(mpv, "chapter-list", &["title", "time"]),
        editions: collection(mpv, "edition-list", &["id", "title", "default"]),
        playlist: collection(
            mpv,
            "playlist",
            &["id", "filename", "title", "current", "playing"],
        ),
    }
}

pub fn chapters(info: &MediaFileInfo) -> Vec<MediaChapter> {
    let mut chapters = info
        .chapters
        .iter()
        .filter_map(|entry| {
            let time_seconds = entry.properties.get("time")?.parse::<f64>().ok()?;
            if !time_seconds.is_finite() || time_seconds < 0.0 {
                return None;
            }
            let title = entry
                .properties
                .get("title")
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("Chapter {}", entry.index + 1));
            Some(MediaChapter {
                index: entry.index,
                title,
                time_seconds,
            })
        })
        .collect::<Vec<_>>();
    chapters.sort_by(|left, right| {
        left.time_seconds
            .total_cmp(&right.time_seconds)
            .then_with(|| left.index.cmp(&right.index))
    });
    chapters
}

pub fn property_keys() -> impl Iterator<Item = &'static str> {
    PROPERTY_GROUPS
        .iter()
        .flat_map(|(_, properties)| properties.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn media_inspector_queries_unique_exact_libmpv_keys() {
        let keys = property_keys().collect::<Vec<_>>();
        let unique = keys.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(keys.len(), unique.len());
        for required in [
            "path",
            "file-format",
            "video-params/pixelformat",
            "audio-params/channels",
            "demuxer-cache-duration",
        ] {
            assert!(unique.contains(required));
        }
    }

    #[test]
    fn typed_chapters_keep_real_titles_sort_times_and_supply_fallbacks() {
        let info = MediaFileInfo {
            chapters: vec![
                MediaCollectionEntry {
                    index: 1,
                    properties: BTreeMap::from([
                        ("title".to_string(), "Act II".to_string()),
                        ("time".to_string(), "12.5".to_string()),
                    ]),
                },
                MediaCollectionEntry {
                    index: 0,
                    properties: BTreeMap::from([("time".to_string(), "0".to_string())]),
                },
                MediaCollectionEntry {
                    index: 2,
                    properties: BTreeMap::from([("time".to_string(), "NaN".to_string())]),
                },
            ],
            ..MediaFileInfo::default()
        };

        let chapters = chapters(&info);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "Chapter 1");
        assert_eq!(chapters[0].time_seconds, 0.0);
        assert_eq!(chapters[1].title, "Act II");
        assert_eq!(chapters[1].time_seconds, 12.5);
    }
}
