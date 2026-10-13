# 10 — Music (later phase)

Scope decision (D28): **video + music**. No photos, podcasts or games planned. Music comes after the video experience is solid.

## Sources (all via the normal provider/addon system)
- **Own music files**: local/NAS library, tags (ID3/Vorbis/FLAC), album art, MusicBrainz IDs, gapless playback, ReplayGain.
- **Music addons**: new protocol types (`artist`, `album`, `track`, `playlist`) in v2, so addons can provide catalogs and streams for music the same way they do for video.
- **Self-hosted servers**: Navidrome/Subsonic (OpenSubsonic API), Jellyfin music, Plex music as source providers.

## Tracking
- **Last.fm and ListenBrainz** as tracker addons on the same tracker/scrobble API as Trakt/Simkl, with music-specific events (track started, scrobble threshold: 50% or 4 min, as Last.fm requires).

## UI
- A music section in the rail (hideable), album/artist/playlist views, queue, mini-player that keeps playing while you browse video catalogs, lyrics as an addon resource (synced LRC).
- TV mode: big artwork "now playing" screen, controllable from the phone remote.

## Player
- mpv handles audio too; additions: gapless, crossfade (optional), ReplayGain, an exclusive/bit-perfect output option on desktop, plus the headphone features from [06](06-ux-input-player.md#player).

## Open questions
- Do music addons need a different discovery UI (radio/mixes) or just catalogs?
- Offline downloads for music (same download system as video)?
