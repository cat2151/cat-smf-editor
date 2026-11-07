# cat-smf-editor

A lightweight application aiming to easily edit SMF (Standard MIDI File). It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

The editing features are idiosyncratic. It's almost exclusively for personal use. Specifications may change tomorrow, potentially making existing files unreadable.

## Download

[Download Windows version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

This is an experimental project, so it may occasionally return 404.

## Past Challenges and What This App Solves
### SMF Editing
- Attempts to publish CC0 licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- Serious performances and their editing often require preparation, such as launching a DAW.
- Lightweight apps for casual performance and editing often had a user experience that was "not for me."
- Nowadays, if you have a desired UX, you can achieve it through "vibecoding."
- So I decided to do just that.
### Enjoying Casual, Simple Composition-like Play
- Existing SMF editors and DAWs often didn't suit my desired workflow (were "not for me").
  - When enjoying casual, simple composition-like play, they were often paid, required account registration, had high hurdles to get audio plugins playing, or high hurdles to produce the first sound.
- Nowadays, if you have a desired UX, you can achieve it through "vibecoding."
- So I decided to do just that.

## Concept, Goals
- Toy
  - It aims to be more of a toy than an editor.
  - The positioning of this app is roughly: [Toy] >> Educational >> Commercial.
  - Only a very limited number of elements can be edited.
  - It has "editor" in its name, but don't mind that.
- Sound Playback
  - It aims to maintain "playback in the author's environment" as much as possible.
- Immediate Playback
  - From app launch to sound playback for a single track is an approximate 0.2 seconds. It prioritizes the fastest possible playback.
    - Processes like GUI display preparation are also handled after the sound plays.
      - Sound playing before the GUI appears might be considered a sound-based splash screen.
- Implement Only My Own Workflow
  - Due to almost zero development resources, this is a triage.
  - Developing this kind of editor requires enormous development costs.
    - For example, to achieve both safe editing (such as undo, time machine, and non-destructive editing) and a standard UX, the development cost is enormous. This includes tasks like directing data model design, operational design, requirements gathering, verification implementation, dogfooding, and making feature choices (which isn't feasible if aiming for a standard, thus requiring an all-inclusive approach).
  - If you are looking for a common "lightweight and standard editor," please look elsewhere.
- From Educational to Experimental
  - It leans more towards educational use than commercial use.
  - However, due to the aforementioned constraints, this app
  - leans more towards a "toy" or "experimental" direction than educational.
- Experimentation, Exploration, Personal Use
  - Destructive changes will be frequent.
    - It will be commonplace for files saved yesterday to be unreadable today.
      - Therefore, please keep usage at a "toy" level.

## Random Thoughts Corner
- SMF editors have many uses and too many considerations. If aiming for a "production-ready level," the development cost becomes enormous.
  - A toy. Want to heavily edit only a few parts: timbre and effects, tempo curves, arpeggios, fine-tuning each note on a piano roll. Prioritize a rapid edit and verification cycle, with fully automatic load and save operations, no manual intervention needed → This app targets only a small subset of such a workflow.
  - Want to quickly browse and play, quickly visualize content. A lightweight browser or viewer.
  - Want to make small changes. Guarantee that everything except the changed part remains exactly original, with diff visualization for that purpose. A lightweight editor.
  - A toy for practicing composition and arrangement.
    - Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but development cost would be enormous.
    - No, I think it's better to compose with other standard, modern apps, not an SMF editor.
  - Serious composition and arrangement.
    - Same as above.
    - Furthermore, highly advanced undo, time machine, and non-destructive editing must be implemented at an extremely high level.
- To be added later.

## Out of Scope, What It Does Not Aim For
- DAW
  - Direct reading, writing, and editing of not only SMF but also DAW projects from other popular DAWs, including Audio.
- Plugins
  - Freely extend all editor functions with plugins.
- Scripting
  - Freely extend all editor functions with scripts.
- Standard
  - 200% coverage of the look and behavior of standard SMF editors and DAWs. A complete superset of features and behavior for all editors and DAWs.
- Features
  - Equipped with every possible feature. Usable for all purposes.
- Compatibility
  - Complete data compatibility.
    - Capable of loading all SMF files, importing all DAW projects.
    - Export compatible with all music data formats and all DAW projects.
  - Complete settings compatibility. Capable of loading and operating with all past settings.
- Robustness
  - Absolutely no bugs no matter what you do. The app never crashes.
- Usability
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - Operates at high speed with no CPU load in any environment. Sound never drops out.
- User Requests
  - Immediately responding to all user requests.

## For Developers: Build Instructions
- Clone the necessary external repositories and build with Cargo.
- More details will be added later.

## License
- MIT