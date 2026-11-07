# cat-smf-editor

A lightweight application designed for easy editing of SMF (Standard MIDI File) files. It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.
Its editing features are somewhat idiosyncratic and it's mostly for personal use. Specifications may change rapidly, potentially making older files unreadable.

## Download

[Download Windows version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

As this is an experimental project, the link may occasionally return a 404 error.

## Previous Challenges and What This App Solves
### SMF Editing
- Attempts to publish CC0 licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- For serious performances or their associated editing, preparations like launching a DAW are often required.
- Lightweight applications for casual playback and editing often had a user experience that was not for me.
- Nowadays, if you have a desired UX, you can achieve it with vibe coding.
- So I decided to do just that.
### Enjoying Casual, Simple Composition-like Play
- Existing SMF editors and DAWs often didn't align with my desired workflow.
- Nowadays, if you have a desired UX, you can achieve it with vibe coding.
- So I decided to do just that.

## Concept, What It Aims For
- A Toy
  - It aims to be more of a toy than an editor.
  - The positioning of this application is roughly [Toy] >> Educational >> Commercial.
  - Only a very limited number of elements can be edited.
  - It has "editor" in its name, but don't mind it.
- Sound Playback
  - It aims to maintain "sounding correctly in the author's environment" as much as possible.
- Immediate Playback
  - From app launch until sound playback, it's a perceived 0.2 seconds for a single track. Playback is optimized for maximum speed.
    - Processes like GUI display preparation are also handled after the sound starts.
      - Sound playing before the GUI is displayed could be called a sound version of a splash screen.
- Implement Only My Own Workflow
  - Due to nearly zero development resources, it's a matter of triage.
  - Developing this kind of editor requires immense development cost.
    - For example, achieving both safe editing (such as undo, time machine, and non-destructive editing) and a standard UX requires directing data model design, operational design, requirements elicitation, verification implementation, and dogfooding. Even just selecting features (which, if aiming for a standard, would mean including everything) incurs enormous development costs.
  - If you are looking for a common "lightweight and standard editor," please look elsewhere.
- From Educational to Experimental
  - It is oriented more towards educational use than commercial use.
  - However, due to the constraints mentioned above, this app is
  - oriented more towards being a toy or for experimental use than educational.
- Experimentation, Exploration, Personal Use
  - Destructive changes will be made frequently.
    - Files saved yesterday becoming unreadable today will be commonplace.
      - Therefore, its use should be limited to a toy level.

## Corner for Random Thoughts
- SMF editors have many uses and too many considerations; if you aim for a 'practical level,' the development cost becomes enormous.
  - A toy. I want to heavily edit only specific parts: fine-tune timbre, effects, tempo curves, arpeggios, and individual notes on a piano roll. Prioritize a quick edit and verification cycle, with fully automated loading and saving (no manual operations). → This app targets only a very small part of that workflow.
  - I want to browse and play smoothly, and quickly visualize content. A lightweight browser or viewer.
  - I want to make minor changes. Ensure that everything except the changed parts remains exactly as it was, with diff visualization for that purpose. A lightweight editor.
  - A toy for practicing composition and arrangement.
    - Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but the development cost would be enormous.
    - No, instead of an SMF editor, I think it's better to compose with other standard, modern applications.
  - Full-fledged composition and arrangement.
    - Same as above.
    - Furthermore, a very high-level implementation of advanced undo, time machine, non-destructive editing, etc., is essential.
- To be added later.

## Out of Scope, What It Does Not Aim For
- DAW
  - Directly reading, writing, and editing not only SMF but also DAW projects and projects from other famous DAWs, including audio, of course.
- Plugins
  - Freely extending all editor functions with plugins.
- Scripting
  - Freely extending all editor functions with scripts.
- Standard
  - 200% coverage of the look and behavior of standard SMF editors and DAWs. Fully compatible with and superseding all editors and DAWs.
- Features
  - Equipped with every conceivable feature. Usable for all purposes.
- Compatibility
  - Full data compatibility.
    - Able to load all SMF files, able to import projects from all DAWs.
    - Export to all music data formats and all DAW projects.
  - Full settings compatibility. Able to load and operate with all past settings.
- Robustness
  - Absolutely no bugs, no matter what you do. The app never crashes.
- Usability
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - High-speed operation with no CPU load in any environment. Sound never drops out.
- User Requests
  - Immediately responding to all user requests.

## For Developers: Build Instructions
- Clone the necessary external repositories, then build with Cargo.
- More details will be added later.

## License
- MIT