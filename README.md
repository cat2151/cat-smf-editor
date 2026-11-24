# cat-smf-editor

A lightweight application aimed at easily editing SMF (Standard MIDI File). It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

The editing features are idiosyncratic; it's almost exclusively for my own use. Specifications might change tomorrow, potentially making previous files unreadable.

## Download

[Download Windows version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

Since this is an experimental project, the link might occasionally return 404.

## Past Challenges and What This App Solves
### SMF Editing
- Initiatives to publish CC0 licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- For full-scale performance and its editing, pre-preparation, such as launching a DAW, is often required.
- Lightweight applications for easy performance and editing often had a user experience that was "not for me."
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding."
- So I decided to do that.
### Enjoying Casual and Simple Composition-like Play
- Existing SMF editors and DAWs often didn't provide the workflow I desired.
  - For casual, simple composition-like play, issues included being paid, requiring account registration, high hurdles to play with audio plugins, high hurdles to get the first sound, etc.
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding."
- So I decided to do that.
### Zundamon
- I wanted to easily enjoy Zundamon vocals. Existing apps often didn't provide the workflow I desired.
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding."
- So I decided to do that.

## Concept, What It Aims For
- **Toy**
  - It aims to be more of a toy than an editor.
  - The positioning of this app is roughly [Toy] >> Educational >> Commercial.
  - Only a very limited number of elements can be edited.
  - "Editor" is in the name, but don't mind it.
- **Sound Playback**
  - It aims to maintain "sound playback in the author's environment" as much as possible.
- **Immediate Sound Playback**
  - For a single track, sound playback from app launch feels like 0.2 seconds. Playback is prioritized for speed.
    - Processes like GUI display preparation are also deferred until after the sound.
      - Sound playing before the GUI appears might be considered a sound-based splash screen.
- **Implement Only My Own Workflow**
  - Due to near-zero development resources, this is a triage situation.
  - Developing this kind of editor requires enormous development costs.
    - For example, just to achieve safe editing and a standard UX with intelligent undo/redo, time travel, non-destructive editing, etc., the development cost is immense when considering data model design, operational design, requirement elicitation, verification implementations, dogfooding, and feature selection (which, if aiming for a standard, means including everything).
  - If you are looking for a common "lightweight and standard editor," please look elsewhere.
- **From Educational to Experimental**
  - It leans more towards educational use than commercial use.
  - However, due to the aforementioned constraints,
  - this app leans more towards being a toy or experimental tool than an educational one.
- **Experimentation, Exploration, Personal Use**
  - Destructive changes are frequent.
    - It will be commonplace for files saved yesterday to be unreadable today.
      - Therefore, please limit its use to a toy level.

## My Casual Thoughts
- **Sound Interruptions**
  - Multi-threaded rendering has been adopted. Sound interruptions have been reduced.
  - Whether to implement a Track Freeze function is under consideration.
- **SMF editors** have various uses and too many considerations; "aiming for a practical level" incurs enormous development costs. For example, to list them:
  - **Toy.** Want to heavily edit only a part. Tweak timbre and effects, tempo curves, arpeggios, and individual notes on a piano roll. Prioritize quick edit-and-verify cycles, with fully automatic load/save without manual operation → This app targets only a small portion of such a workflow.
  - Want to smoothly browse and play, smoothly visualize content. A lightweight browser or viewer.
  - Want to make a small change. Guarantee that everything outside the changed part remains perfectly original, with diff visualization for that purpose. A lightweight editor.
  - **A toy for practicing composition/arrangement.**
    - Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but it incurs huge development costs.
    - No, I think it's better to compose with other standard modern applications rather than an SMF editor.
  - **Full-scale composition/arrangement.**
    - Same as above.
    - Furthermore, highly advanced undo, time machine, non-destructive editing, etc., must be implemented to an extremely high standard.
- To be added later.

## Out of Scope, Not Aimed For
- **A standard or masterpiece as a beginner's DAW or educational DAW.**
  - That even cats and babies can perfectly master all its functions immediately, and it also serves as preparation for using more advanced DAWs.
- **DAW**
  - Not just SMF, but also direct reading/writing and editing of DAW projects and projects from other famous DAWs, including audio.
- **Plugins, Add-ons, Add-ins**
  - Freely extending all editor functions with external DLLs.
- **Script**
  - Freely extending all editor functions with scripts.
- **Standard**
  - 200% coverage of the look and behavior of standard SMF editors and DAWs. Fully superior compatible with all editors and DAWs.
- **Features**
  - Equipped with every conceivable feature. Usable for all purposes.
- **Compatibility**
  - **Full data compatibility.**
    - Capable of loading all SMFs, capable of importing all DAW projects.
    - Export supports all music data and all DAW projects.
  - **Full settings compatibility.** Works by loading all past settings.
- **Robustness**
  - Absolutely no bugs no matter what you do. The app never crashes.
- **Usability**
  - Pursuit of convenience and ease of use to the extreme.
- **Performance**
  - High-speed operation without CPU load in all environments. Sound never cuts out.
- **Requests**
  - Instant response to all user requests.

## Additional Notes
- **Elaboration on the Concept**
  - Playing with rich timbres and easy input using SMF.
    - Richer than the bundled sounds of typical SMF editors (because it can fully play VST3/CLAP timbres).
    - Easier to play VST3/CLAP than typical SMF editors (because it directly hosts audio plugins without complex MIDI routing).
    - More intuitive operation to adjust note positions and durations than MML-only (because it can be adjusted on a piano roll screen).
    - Easier and clearer to visualize and edit songs, and easier and clearer to combine and play with audio plugins than DAWs (because without audio, it specializes in understanding MIDI notes and ease of simple input).
    - Easier input than MML or certain DAWs (because it allows MML, step sequencer, virtual MIDI keyboard, and easily arranging small patterns on the song screen).
    - Faster startup than DAWs (because sound plays in a perceived 0.2 seconds for a single track).
    - More direct MML input and Zundamon vocal performance than DAWs (because this app directly supports MML input and lyric input).
  - **Usage Differentiation**
    - With typical SMF editors, SMF editing is more stable, and you can connect to any external MIDI device.
    - With browser DAWs, sound plays regardless of the device.
    - With DAWs, audio (e.g., recorded vocals) can be handled in an integrated manner.
    - With more advanced MML, expression is higher, and complex articulations can be achieved.
    - With more advanced DAWs (especially paid ones), they are more functional, higher-performing, have fewer bugs, operate stably, and allow for efficient creation of high-quality music.

## For Developers: Build Instructions
- Clone the necessary external repositories and build with Cargo.
- More details will be written later.

## License
- MIT