# cat-smf-editor

A lightweight application aiming to easily edit SMF (Standard MIDI File) files. It utilizes [UAPMD](https://github.com/atsushieno/uapmd). It is written in Rust.

The editing features are quite idiosyncratic. It's almost exclusively for my own use. Specifications might change tomorrow, potentially making previous files unreadable.

## Download

[Download Windows version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

As it's an experimental project, it might occasionally return a 404 error.

## Past Challenges and What This App Solves
### SMF Editing
- Attempts to publish CC0 licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- For full-scale performance and editing, pre-preparation, such as launching a DAW, is often required.
- Lightweight apps for easy performance and editing often had a user experience that was not for me.
- Nowadays, if you have a desired UX, you can achieve it through 'vibe coding'.
- So, I decided to do just that.
### Enjoying Casual, Simple Composition-like Play
- Existing SMF editors and DAWs often had workflows that were not for me.
  - For casual, simple composition-like play, they were often paid, required account registration, had high hurdles to get audio plugins to play, or high hurdles to make the first sound, etc.
- Nowadays, if you have a desired UX, you can achieve it through 'vibe coding'.
- So, I decided to do just that.
## Zundamon
- I wanted to easily enjoy Zundamon singing. Existing apps often had workflows that were not for me.
- Nowadays, if you have a desired UX, you can achieve it through 'vibe coding'.
- So, I decided to do just that.

## Concept, What It Aims For
- A Toy
  - It aims to be more of a toy than an editor.
  - The positioning of this app is roughly [Toy] >> Educational >> Commercial.
  - Only a very limited number of elements can be edited.
  - It has 'editor' in its name, but don't mind that.
- Playback Capability
  - It aims to maintain "playback in the author's environment" as much as possible.
- Immediate Playback
  - From app launch to sound playback, for a single track, it's perceptibly 0.2 seconds. Playback is prioritized for speed.
    - Processes like GUI display preparation are also handled *after* the sound.
    - Sound playing before the GUI appears might be considered a sound version of a splash screen.
- Implement Only My Own Workflow
  - Due to nearly zero development resources, it's a triage.
  - Developing this kind of editor requires enormous development costs.
    - For example, achieving both safe editing and a standard UX with features like intelligent undo/redo, time machine, and non-destructive editing requires huge development costs just for directing data model design, operational design, requirements elicitation, validation implementation, dogfooding, and feature selection (which, if aiming for a standard, means including everything).
  - If you're looking for a common "lightweight, standard editor," please look elsewhere.
- From Educational to Experimental
  - It leans more towards educational use than commercial use.
  - However, due to the aforementioned constraints, this app...
  - ...leans more towards being a toy or experimental tool than educational.
- Experimentation, Exploration, Personal Use
  - It undergoes frequent destructive changes.
    - It will be commonplace for files saved yesterday not to be loadable today.
    - Therefore, please keep its usage to a "toy" level.

## Random Thoughts Corner
- Audio Dropouts
  - Multithreaded rendering has been adopted. Audio dropouts have been reduced.
  - Whether to implement a Track Freeze feature is currently under consideration.
- SMF editors have various uses and too many considerations; aiming for a "practical level" leads to enormous development costs. For example, a list might include:
  - A toy. Want to heavily edit only a portion. Finely adjust timbre and effects, tempo curves, arpeggios, and individual notes on a piano roll. Prioritize quick edit-and-verify cycles, with fully automatic load/save without manual operation → This app targets only a small part of such a workflow.
  - Want to quickly browse and play, quickly visualize content. A lightweight browser or viewer.
  - Want to make a small change. Ensure that everything except the modified part remains completely original, with diff visualization for this purpose. A lightweight editor.
  - A toy for practicing composition and arrangement.
    - Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but development costs would be enormous.
    - No, I think it's better to compose with other standard modern apps rather than an SMF editor.
  - Serious composition and arrangement.
    - Same as above.
    - Furthermore, highly advanced undo, time machine, non-destructive editing, etc., must be implemented to an extremely high standard.
- To be added later.

## Out of Scope, What It Does Not Aim For
- A standard, masterpiece beginner DAW or educational DAW.
  - Something that even a cat or baby can perfectly master all features immediately, and also serves as preparation for using more advanced DAWs.
- DAW.
  - Directly reading, writing, and editing not just SMF but also DAW projects from other famous DAWs, and of course, audio.
- Plugins, Add-ons, Add-ins.
  - Freely extending all editor functions with external DLLs.
- Script.
  - Freely extending all editor functions with scripts.
- Standard.
  - 200% coverage of the look and behavior of standard SMF editors and DAWs. Fully backward and forward compatible with all editors and DAWs.
- Features.
  - Equipped with every conceivable feature. Usable for all purposes.
- Compatibility.
  - Complete data compatibility.
    - Capable of loading all SMF files, importing all DAW projects.
    - Export supports all music data formats, all DAW projects.
  - Complete settings compatibility. Loads and operates with all past settings.
- Robustness.
  - Absolutely no bugs regardless of actions. The app never crashes.
- Usability/Convenience.
  - Pursuing ultimate convenience and ease of use.
- Performance.
  - High-speed operation with no CPU load in any environment. Audio never drops out.
- Requests.
  - Immediately responding to all user requests.

## Supplementary Notes
- Supplementary Image/Concept
  - Playing with rich timbres and easy input using SMF.
    - Richer than the bundled timbres of general SMF editors (because it can fully play VST3/CLAP timbres).
    - Easier to play VST3/CLAP than general SMF editors (because it directly hosts audio plugins without complex MIDI routing).
    - More intuitive control over note position and duration than MML-only (because it can be adjusted on a piano roll screen).
    - Easier and clearer to visualize/edit music and combine audio plugins for play than a DAW (because it specializes in easy MIDI note comprehension and simple input, lacking audio functionality).
    - Easier input than MML or certain DAWs (because it allows MML, step sequencers, virtual MIDI keyboards, and easy placement of small patterns on a song screen).
    - Faster startup than a DAW (because sound plays in a perceptibly 0.2 seconds for a single track).
    - More direct operation for MML input and Zundamon singing than a DAW (because the DAW itself directly supports MML input and lyric input).
- Differentiation/Usage Scenarios
  - With a general SMF editor, you can edit SMF more stably and connect to any external MIDI device.
  - With a browser DAW, it plays regardless of the device.
  - With a DAW, audio (like recorded vocals) can be handled in an integrated manner.
  - With more advanced MML, higher expressiveness and complex articulations can be achieved.
  - With more advanced DAWs (especially paid ones), you can create high-quality music more efficiently, with more features, higher performance, fewer bugs, and stable operation.

## For Developers: Build Instructions
- Clone the necessary external repositories and build with Cargo.
- More details will be provided later.

## License
- MIT