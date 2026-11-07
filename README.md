# cat-smf-editor

A lightweight application aiming to make editing SMF (Standard MIDI File) easy. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

The editing features are quite idiosyncratic. It's almost exclusively for my personal use. Specifications might change tomorrow, potentially making previous files unreadable.

## Download

[Download Windows version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

This is an experimental project, so the link might occasionally return 404.

## Past Challenges and What This App Solves
### SMF Editing
- Attempts to publish CC0 licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- For serious performance and editing, it often requires preparation, such as launching a DAW.
- Lightweight apps for casual playing and editing often had a user experience that was "not for me."
- Nowadays, if you have a desired UX, you can achieve it through vibe coding.
- So, that's what I decided to do.
### Enjoying Casual, Simple Composition-like Play
- Existing SMF editors and DAWs often didn't align with my desired workflow.
  - When casually enjoying simple composition-like play, hurdles included: being paid, requiring account registration, high barrier to playing with audio plugins, high barrier to making the first sound, etc.
- Nowadays, if you have a desired UX, you can achieve it through vibe coding.
- So, that's what I decided to do.

## Concept, What It Aims For
- Toy
  - Aims to be more of a toy than an editor.
  - The positioning of this app is roughly [Toy] >> Educational >> Commercial.
  - Only a very small number of elements can be edited.
  - Don't mind that "editor" is in the name.
- Sound Playback
  - Aims to maintain "sound playback in the author's environment" as much as possible.
- Immediate Sound Playback
  - From app launch to sound playback for a single track, it's a perceived 0.2 seconds. Playback is prioritized to be as fast as possible.
    - Even GUI display preparation and other processing occur *after* the sound.
      - Sound playing before the GUI appears might be considered a sound version of a splash screen.
- Implement Only My Own Workflow
  - Due to almost zero development resources, it's a matter of triage.
  - Developing this type of editor requires enormous development costs.
    - For example, just achieving intelligent undo/redo, time machine functionality, non-destructive editing, etc., to balance safe editing with a standard UX, requires immense development cost for data model design, operational design, requirements elicitation, validation implementation, dogfooding, and deciding which features to include (which, if aiming for standard, means including everything).
  - If you are looking for a common "lightweight and standard editor," please look elsewhere.
- From Educational to Experimental
  - It leans more towards educational use than commercial use.
  - However, due to the constraints mentioned above, this app leans more towards a toy or experimental tool than educational.
- Experimentation, Exploration, Personal Use
  - Destructive changes will be frequent.
    - It will be commonplace for files saved yesterday to be unreadable today.
      - Therefore, please limit its use to a toy level.

## Rough Notes Corner
- SMF editors serve various purposes and have too many considerations, making development costs enormous "if aiming for a practical level." For example, if I were to list them:
  -   **Toy.** Wants to edit only a specific part intensively. Fine-tuning timbre, effects, tempo curves, arpeggios, and individual notes on a piano roll. Prioritizes a quick edit-and-verify cycle, with fully automatic load/save without manual intervention. → The workflow targeted by this application is only a small part of this.
  -   **Quick browsing and playback, quick visualization of content.** A lightweight browser or viewer.
  -   **Slight modification.** Guarantees that parts other than the modified section remain completely original, with diff visualization for this purpose. A lightweight editor.
  -   **Toy for practicing composition/arrangement.**
    -   Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but development costs would be huge.
    -   However, I believe it's better to compose music with standard, modern applications other than SMF editors.
  -   **Serious composition/arrangement.**
    -   Same as above.
    -   Furthermore, extremely high-level implementation of advanced undo, time machine, and non-destructive editing is essential.
- To be added later.

## Out of Scope, Not Aimed For
- A standard or masterpiece as an entry-level DAW or educational DAW.
  - Immediately enables even cats and babies to perfectly master all features, and serves as preparation for using more advanced DAWs.
- A DAW.
  - Not just SMF, but also directly reading, writing, and editing DAW projects and other famous DAW projects, including Audio.
- Plugins, add-ons, add-ins.
  - Freely extending all editor functions with external DLLs.
- Scripting.
  - Freely extending all editor functions with scripts.
- Standard.
  - Completely covers 200% of the appearance and behavior of standard SMF editors and DAWs. Fully backward and forward compatible with all editors and DAWs.
- Features.
  - Equipped with every imaginable feature. Usable for all purposes.
- Compatibility.
  - Complete data compatibility.
    - Able to load all SMFs, import all DAW projects.
    - Exports to all music data formats and all DAW projects.
  - Complete setting compatibility. Loads and operates with all past settings.
- Robustness.
  - Absolutely no bugs, no matter what you do. The app never crashes.
- Convenience.
  - Pursues convenience and ease of use to the extreme.
- Performance.
  - Operates at high speed with no CPU load in any environment. Audio playback is never interrupted.
- Requests.
  - Immediately responds to all user requests.

## For Developers: Build Instructions
- Clone the necessary external repositories and build with cargo.
- Details will be added later.

## License
- MIT