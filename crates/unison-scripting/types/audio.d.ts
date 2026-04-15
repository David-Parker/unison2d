/// <reference types="@typescript-to-lua/language-extensions" />

// ===================================================================
// Audio types — spatial + non-spatial playback, buses, music
// ===================================================================

/** Opaque sound ID returned by unison.assets.load_sound. */
declare type SoundId    = number;

/** Opaque playback handle returned by play / play_music / world:play_sound_at. */
declare type PlaybackId = number;

/** Distance-attenuation model for spatial sounds. */
declare type Rolloff    = "linear" | "inverse";

/** Options accepted by unison.audio.play. */
declare interface PlayOptions {
  /** Named bus to route through. Defaults to "sfx". */
  bus?:     string;
  /** Volume scalar; 1.0 = unity gain. */
  volume?:  number;
  /** Pitch scalar; 1.0 = original pitch. */
  pitch?:   number;
  /** Whether playback loops when it reaches the end. */
  looping?: boolean;
  /** Fade-in duration in seconds. */
  fade_in?: number;
}

/** Options accepted by world:play_sound_at — PlayOptions plus spatial params. */
declare interface PlayAtOptions extends PlayOptions {
  /** Distance beyond which the sound is silent. */
  max_distance?: number;
  /** Distance-attenuation curve. Defaults to "inverse" (inverse-square). */
  rolloff?:      Rolloff;
}

/** Options accepted by unison.audio.play_music. */
declare interface MusicOptions {
  /** Named bus to route through. Defaults to "music". */
  bus?:       string;
  /** Volume scalar; 1.0 = unity gain. */
  volume?:    number;
  /** Fade-in duration in seconds. */
  fade_in?:   number;
  /** Crossfade duration in seconds when replacing the current track. */
  crossfade?: number;
}

/** Options for tweening volume / pitch / master / bus volume changes. */
declare interface AudioTweenOptions { tween?: number; }

/** Options for fading a playback out on stop. */
declare interface AudioStopOptions  { fade_out?: number; }

/** Audio service — playback, buses, music. (Loading lives on unison.assets.load_sound.) */
declare interface UnisonAudio {
  /** Free a previously loaded sound. */
  unload(this: void, sound: SoundId): void;

  /** Play a non-positional sound. Returns a PlaybackId (0 if deferred on web before the first user gesture). */
  play(this: void, sound: SoundId, opts?: PlayOptions): PlaybackId;
  /** Stop a playback, optionally fading out. */
  stop(this: void, playback: PlaybackId, opts?: AudioStopOptions): void;
  /** Pause a playback (keeps its handle valid). */
  pause(this: void, playback: PlaybackId): void;
  /** Resume a paused playback. */
  resume(this: void, playback: PlaybackId): void;
  /** True if the playback is currently producing audio. */
  is_playing(this: void, playback: PlaybackId): boolean;

  /** Update a playback's volume, optionally tweened over `tween` seconds. */
  set_volume(this: void, playback: PlaybackId, volume: number, opts?: AudioTweenOptions): void;
  /** Update a playback's pitch, optionally tweened over `tween` seconds. */
  set_pitch(this: void, playback: PlaybackId, pitch: number, opts?: AudioTweenOptions): void;

  /** Start a music track (exclusive — replaces any current track, with optional crossfade). */
  play_music(this: void, sound: SoundId, opts?: MusicOptions): PlaybackId;
  /** Stop the current music track. */
  stop_music(this: void, opts?: AudioStopOptions): void;
  /** Pause the current music track. */
  pause_music(this: void): void;
  /** Resume the current music track. */
  resume_music(this: void): void;
  /** Returns the currently-playing music PlaybackId, or undefined if none. */
  current_music(this: void): PlaybackId | undefined;

  /** Set the master volume (applies on top of all bus volumes). */
  set_master_volume(this: void, v: number, opts?: AudioTweenOptions): void;
  /** Set the named bus volume. Unknown bus names are a no-op. */
  set_bus_volume(this: void, bus: string, v: number, opts?: AudioTweenOptions): void;
  /** Create a named bus. Idempotent — repeated calls with the same name are no-ops. */
  create_bus(this: void, name: string): void;

  /** Stop all non-spatial playbacks. */
  stop_all(this: void, opts?: AudioStopOptions): void;
}
