# cat-smf-editor

A lightweight application aiming to easily edit SMF (Standard MIDI File) files. It utilizes [UAPMD](https://github.com/atsushieno/uapmd). Written in Rust.

Its editing features are peculiar. It's almost exclusively for personal use. Specifications might change tomorrow, potentially making previous files unreadable.

## Download

Currently under construction, download is not available.

The link below is currently 404. It's kept for now so it can be accessed quickly once restored.

[Download Windows version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

## Past Challenges and What This App Solves
### SMF Editing
- Initiatives to publish CC0 licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- For serious performance and editing, pre-setup such as launching a DAW is often required.
- Lightweight apps for easy performance and editing often had a user experience that was "not for me".
- In modern times, if you have a desired UX, you can achieve it with "vibe coding".
- I decided to do just that.
### Enjoying Casual, Simple Composition-like Play
- Existing SMF editors and DAWs often didn't offer the workflow I desired.
- In modern times, if you have a desired UX, you can achieve it with "vibe coding".
- I decided to do just that.

## Concept, What It Aims For
- Toy
  - It aims to be more of a toy than an editor.
  - The positioning of this app is roughly: [Toy] >> Educational >> Commercial.
  - Only a very limited number of elements can be edited.
  - "editor" is in the name, but don't mind it.
- Sound Output
  - The goal is to maintain "it plays in the author's environment" as much as possible.
- Immediate Sound Output
  - From app launch to sound output, it feels like 0.2 seconds for a single track. Playback is optimized for maximum speed.
    - Processes like GUI display preparation are also done after the sound.
    - Sound playing before the GUI appears might be considered a sound version of a splash screen.
- Implementing Only the Workflows I Use
  - Due to almost zero development resources, this is a triage.
  - Developing this type of editor requires enormous development costs.
    - For example, just directing data model design, operational design, feature identification, verification implementations, and dogfooding to balance safe editing (like undo, time machine, non-destructive editing) with a standard UX, and then choosing features (which is impossible if you aim for standard, so it becomes all-inclusive), these alone incur enormous development costs.
  - If you are looking for a common "lightweight and standard editor," please look elsewhere.
- From Educational to Experimental
  - It leans more towards educational use than commercial use.
  - However, due to the aforementioned constraints of this app,
  - it leans more towards "toy" or "experimental" use than educational.
- Experimentation, Exploration, Personal Use
  - Destructive changes will be frequent.
    - It will be common for files saved yesterday not to be loadable today.
    - Therefore, please limit its use to a toy level.

## Rough Notes Corner
- SMF editors have many uses and too many considerations; aiming for a "practical level" incurs excessive development costs.
  - A toy. I want to heavily edit only specific parts: timbre and effects, tempo curves, arpeggios, fine-tuning individual notes on a piano roll. Prioritize a quick edit and verification cycle, with fully automatic load and save without manual intervention. → The workflow targeted by this app is a small subset of this.
  - I want to quickly browse and play, quickly visualize content. A lightweight browser or viewer.
  - I want to make a small change. Guarantee that everything outside the changed part remains completely original, with diff visualization for this purpose. A lightweight editor.
  - A toy for practicing composition and arrangement.
    - Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but it incurs enormous development costs.
    - No, I think it's better to compose with standard, modern alternative apps, not an SMF editor.
  - Serious composition and arrangement.
    - Same as above.
    - Furthermore, a very high level of implementation for advanced undo, time machine, non-destructive editing, etc., is essential.
- To be added later.

## Out of Scope, What It Does Not Aim For
- DAW
  - Not just SMF, but directly reading/writing and editing project files of other famous DAWs, and of course, audio.
- Plugins
  - Freely extending all editor functions with plugins.
- Scripts
  - Freely extending all editor functions with scripts.
- Standard
  - 200% coverage of the look and behavior of standard SMF editors and DAWs. Complete superset compatibility with all editors and DAWs.
- Features
  - Equipped with every conceivable feature. Usable for all purposes.
- Compatibility
  - Full data compatibility.
    - Ability to load all SMF files, import projects from all DAWs.
    - Export support for all music data formats and all DAW projects.
  - Full settings compatibility. Operates by loading all past settings.
- Robustness
  - Absolutely no bugs no matter what is done. The app never crashes.
- Convenience
  - Extreme pursuit of convenience and ease of use.
- Performance
  - High-speed operation with no CPU load in any environment. Sound never cuts out.
- Requests
  - Immediate response to all user requests.

## Build Instructions for Developers
- Clone the necessary external repositories and build with cargo.
- More details will be written later.

## License
- MIT