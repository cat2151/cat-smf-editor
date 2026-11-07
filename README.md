# cat-smf-editor

A lightweight application aiming to provide easy editing of SMF (Standard MIDI File). It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

It has peculiar editing features. It's almost exclusively for my personal use. Be warned that specifications might change tomorrow, which could make existing files unreadable.

## Download

[Download Windows version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

This is an experimental project, so it might occasionally return a 404 error.

## Challenges So Far and What This App Solves
### SMF Editing
- Attempts to publish CC0 licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- For serious performances and their associated editing, preparations like launching a DAW are often required.
- Lightweight applications for easy performance and editing often had a user experience that wasn't for me.
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding."
- So, that's what I decided to do.
### Enjoying Casual, Simple Composition-like Play
- Existing SMF editors and DAWs often didn't provide the workflow I desired.
  - When enjoying casual, simple composition-like play, they were often paid, required account registration, had high hurdles to performance with audio plugins, and high hurdles to making the first sound, etc.
- Nowadays, if you have a desired UX, you can achieve it through "vibe coding."
- So, that's what I decided to do.

## Concept and Goals
- Toy
  - It aims to be more of a toy than an editor.
  - The app's positioning is roughly [Toy] >> Educational >> Commercial.
  - Only a very limited number of elements can be edited.
  - It has "editor" in its name, but don't mind that.
- Playability
  - It aims to maintain "sound output in the author's environment" as much as possible.
- Instant Sound
  - From app launch to sound output for a single track, it's perceptibly 0.2 seconds. It's optimized for the fastest possible playback.
    - Processes like GUI display preparation are also handled after the sound.
      - Sound playing before the GUI appears might be considered a sound-based splash screen.
- Implement Only Workflows I Use
  - Due to almost zero development resources, this is a triage situation.
  - Developing this type of editor requires enormous development costs.
    - For example, achieving both safe editing and a standard UX with features like intelligent undo/redo, time-machine functionality, and non-destructive editing would incur immense development costs just for data model design, operational design, requirements elicitation, validation implementation, dogfooding, and directing feature selection (which, if aiming for a standard, would essentially mean implementing everything).
  - If you are looking for a common "lightweight and standard editor," please look elsewhere.
- From Educational to Experimental
  - It is oriented more towards educational use than commercial use.
  - However, due to the aforementioned constraints,
  - this app leans more towards being a toy or experimental tool than an educational one.
- Experimentation, Exploration, Personal Use
  - Destructive changes will be made frequently.
    - It will be commonplace for files saved yesterday to be unreadable today.
      - Therefore, please limit its use to a toy level.

## Random Musings
- SMF editors have many uses and too many considerations; aiming for a "practical level" results in enormous development costs. For example, a list would include:
  - Toy. Want to heavily edit only specific parts. Adjust timbre, effects, tempo curves, arpeggios, and individual notes on a piano roll. Prioritize quick edit-and-verify cycles, with fully automatic load/save without manual operations → A very small part of this workflow is targeted by this app.
  - Want to quickly browse and play, quickly visualize content. A lightweight browser or viewer.
  - Want to make minor changes. Ensure that everything except the changed part remains perfectly original, with diff visualization for that purpose. A lightweight editor.
  - Toy for composition/arrangement practice
    - Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but development costs would be enormous.
    - No, I think it's better to compose with standard modern applications other than an SMF editor.
  - Serious composition/arrangement
    - Same as above.
    - Furthermore, highly advanced undo, time-machine functionality, and non-destructive editing are essential.
- To be added later.

## Out of Scope, Non-Goals
- A standard or masterpiece as a beginner's DAW or educational DAW.
  - One that even cats or babies can perfectly master all functions immediately, and also serves as preparation for using more advanced DAWs.
- DAW
  - Directly reading, writing, and editing not only SMF but also DAW projects from other popular DAWs, including Audio.
- Plugins
  - Freely extending all editor functions with plugins.
- Scripting
  - Freely extending all editor functions with scripts.
- Standard
  - 200% coverage of the appearance and behavior of standard SMF editors and DAWs. Completely superseding all other editors and DAWs.
- Features
  - Equipped with every conceivable feature. Usable for all purposes.
- Compatibility
  - Full data compatibility
    - Capable of loading all SMFs, importing all DAW projects.
    - Exporting to all music data formats and all DAW projects.
  - Full settings compatibility. Capable of loading and operating with any past settings.
- Robustness
  - Absolutely bug-free regardless of input. The app never crashes.
- Usability
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - High-speed operation with no CPU load in any environment. Sound never breaks.
- User Requests
  - Immediately responding to all user requests.

## For Developers: Build Instructions
- Clone the necessary external repositories and build with cargo.
- More details will be written later.

## License
- MIT