pub(crate) fn redact_source(source: &str) -> String {
    let Some(scheme_end) = source.find("://").map(|index| index + 3) else {
        return source.to_owned();
    };
    let scheme = &source[..scheme_end];
    if !matches!(scheme, "rtsp://" | "rtsps://") {
        return source.to_owned();
    }
    let remainder = &source[scheme_end..];
    let Some(at) = remainder.find('@') else {
        return source.to_owned();
    };
    format!("{scheme}***@{}", &remainder[at + 1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_rtsp_credentials_without_changing_the_host() {
        assert_eq!(
            redact_source("rtsp://operator:secret@192.168.1.20/stream"),
            "rtsp://***@192.168.1.20/stream"
        );
        assert_eq!(redact_source("video.mp4"), "video.mp4");
    }
}
