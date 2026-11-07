# cat-smf-editor

A lightweight application aiming to easily edit SMF (Standard MIDI File). It utilizes [UAPMD](https://github.com/atsushieno/uapmd). Written in Rust.

The editing features are idiosyncratic. It's almost exclusively for personal use. Specifications might change tomorrow, potentially making previous files unreadable.

## Download

[Download Windows Version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

This is an experimental project, so it might occasionally return a 404 error.

## Past Challenges and What This App Solves
### SMF Editing
- Efforts to publish CC0-licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- For serious performance or editing, it often requires preparation, such as launching a DAW.
- Lightweight apps for easy performance and editing often had a user experience that was "not for me."
- In modern times, if you have a desired UX, you can achieve it through "vibecoding."
- That's what I decided to do.
### Enjoying Casual, Simple Composition-like Play
- Existing SMF editors and DAWs were "not for me" when it came to realizing my desired workflow.
  - For casual, simple composition-like play, issues included being paid, requiring account registration, high hurdles to playback with audio plugins, and high hurdles to making the first sound.
- In modern times, if you have a desired UX, you can achieve it through "vibecoding."
- That's what I decided to do.

## Concept, What It Aims For
- Toy
  - It aims to be more of a toy than an editor.
  - The positioning of this app is roughly [Toy] >> Educational >> Commercial.
  - Only a very small number of elements can be edited.
  - Don't mind that "editor" is in the name.
- Playback
  - It aims to maintain "playing back in the author's environment" as much as possible.
- Instant Playback
  - For a single track, it feels like 0.2 seconds from app launch until sound plays. Playback is prioritized for speed.
    - Processes like GUI display preparation are also handled *after* the sound plays.
      - Sound playing before the GUI appears might be considered a sound version of a splash screen.
- Implement only the workflows I use
  - Due to almost zero development resources, it's a triage situation.
  - Developing this kind of editor requires immense development costs.
    - For example, achieving both safe editing and a standard UX with intelligent undo/redo, time machine functionality, and non-destructive editing would involve enormous development costs just for things like data model design, operational design, requirements gathering, validation implementations, dogfooding, and feature selection (which, if aiming for standard, means including everything).
  - If you are looking for a common "lightweight and standard editor," please look elsewhere.
- From Educational to Experimental
  - It leans more towards educational use than commercial use.
  - However, due to the above constraints, this app
  - ...leans more towards being a toy or for experimental use, rather than educational.
- Experimentation, Exploration, Personal Use
  - Destructive changes will be made frequently.
    - It will be a common occurrence for files saved yesterday to be unreadable today.
      - Therefore, please limit its use to a toy level.

## Corner for Random Thoughts
- SMF editors have various uses and too many considerations; aiming for a "practical level" results in enormous development costs.
  - A toy. I want to thoroughly edit only certain parts. Tweak timbre, effects, tempo curves, arpeggios, and individual notes on a piano roll. Prioritize a quick edit-and-verify cycle, with fully automatic load and save without manual intervention -> This app targets only a small part of that workflow.
  - I want to quickly browse and play, quickly visualize content. A lightweight browser or viewer.
  - I want to make a small change. Guarantee that everything outside the changed part remains exactly as before, with diff visualization for that purpose. A lightweight editor.
  - A toy for practicing composition and arrangement.
    - Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but development costs would be enormous.
    - No, I think it's better to compose with other standard, modern applications rather than an SMF editor.
  - Serious composition and arrangement.
    - Same as above.
    - Furthermore, high-level implementation of advanced undo, time machine, and non-destructive editing is essential.
- To be added later.

## Out of Scope, What It Does Not Aim For
- DAW
  - Not just SMF, but direct read/write and editing of DAW projects and projects from other famous DAWs, including Audio.
- Plugins
  - Freely extend all editor functions with plugins.
- Scripting
  - Freely extend all editor functions with scripts.
- Standard
  - 200% coverage of the look and behavior of standard SMF editors and DAWs. Fully encompassing and compatible with all editors and DAWs.
- Features
  - Equipped with every conceivable feature. Usable for all purposes.
- Compatibility
  - Full data compatibility.
    - Able to load all SMFs, import all DAW projects.
    - Export supports all music data, all DAW projects.
  - Full settings compatibility. Able to load and operate with all past settings.
- Robustness
  - Absolutely no bugs regardless of input. The app never crashes.
- Usability
  - Extreme pursuit of convenience and ease of use.
- Performance
  - High-speed operation with no CPU load in any environment. Sound never breaks.
- Requests
  - Immediate response to all user requests.

## For Developers: Build Instructions
- Clone the necessary external repositories and build with Cargo.
- More details will be added later.

## License
- MIT