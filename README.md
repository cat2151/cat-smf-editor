# cat-smf-editor

A lightweight application aiming to easily edit SMF (Standard MIDI File). It utilizes [UAPMD](https://github.com/atsushieno/uapmd). Written in Rust.

The editing features have their quirks. It's almost exclusively for personal use. Specifications might change tomorrow, potentially making existing files unreadable.

## Download

Currently under construction, not available for download.

The link below is currently a 404. I'm leaving it for now so it's readily available once it's restored.

[Download Windows version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

## Past Challenges and What This Application Solves
### SMF Editing
- Attempts to publish CC0 licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- Serious performance and editing often require preparation, such as launching a DAW.
- Lightweight apps for easy performance and editing often had a user experience that was "not for me".
- In modern times, if you have a desired UX, you can achieve it through "vibe coding".
- That's what I decided to do.
### Enjoying Casual Composition-like Play
- Existing SMF editors and DAWs often had a workflow that was "not for me" when trying to achieve my desired workflow.
- In modern times, if you have a desired UX, you can achieve it through "vibe coding".
- That's what I decided to do.

## Concept, What It Aims For
- Toy
  - It aims to be more of a toy than an editor.
  - The positioning of this app is roughly: [Toy] >> Educational >> Commercial.
  - Only a very limited number of elements can be edited.
  - It has "editor" in its name, but don't mind that.
- Playing Sound
  - It aims to maintain "playing sound in the author's environment" as much as possible.
- Immediate Sound Playback
  - From app launch to sound playback, it's a perceived 0.2 seconds for a single track. Playback is prioritized for speed.
    - Processes like GUI display preparation are also handled after the sound.
      - Sound playing before the GUI appears might be considered a sound-based splash screen.
- Implement Only Workflows I Use
  - This is triage due to almost zero development resources.
  - Developing this kind of editor requires enormous development costs.
    - For example, just to achieve both safe editing and a standard UX with features like undo, time machine, and non-destructive editing, the development cost is immense. This includes directing data model design, operational design, requirement elicitation, validation implementation, and dogfooding to select features (which becomes impossible if aiming for a standard, thus requiring an all-inclusive approach).
  - If you are looking for a common "lightweight and standard editor", please look elsewhere.
- From Educational to Experimental
  - It leans more towards educational use than commercial use.
  - However, due to the above constraints, this app
  - leans more towards being a toy or for experimental use rather than educational.
- Experimentation, Exploration, Personal Use
  - Destructive changes will be frequent.
    - It will be common for a file saved yesterday to be unreadable today.
      - Therefore, please limit its use to a "toy" level.

## Corner for Random Thoughts
- SMF editors have various uses and too many considerations; aiming for a "practical level" leads to enormous development costs.
  - Toy. I want to edit only specific parts thoroughly. Fine-tune timbre and effects, tempo curves, arpeggios, and individual notes on a piano roll. Prioritize a rapid edit and verification cycle, with fully automated load and save operations, no manual intervention needed. → This app targets only a small part of that workflow.
  - I want to browse and play quickly, and visualize content swiftly. A lightweight browser or viewer.
  - I want to make minor changes. Ensure that everything but the modified parts remains exactly original, with diff visualization for that purpose. A lightweight editor.
  - A toy for practicing composition and arrangement.
    - Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but it involves immense development costs.
    - No, I think it's better to compose with standard, modern alternative apps rather than an SMF editor.
  - Serious composition and arrangement.
    - Same as above.
    - Furthermore, high-level implementation of advanced undo, time machine, non-destructive editing, etc., is essential.
- To be added later.

## Out of Scope, What It Does Not Aim For
- DAW
  - Not just SMF, but direct reading, writing, and editing of DAW projects from famous DAWs, including Audio.
- Plugins
  - Freely extending all editor functions with plugins.
- Scripting
  - Freely extending all editor functions with scripts.
- Standard
  - 200% coverage of the look and behavior of standard SMF editors and DAWs. Fully backward and forward compatible with all editors and DAWs.
- Features
  - Equipped with every imaginable feature. Usable for all purposes.
- Compatibility
  - Full data compatibility
    - Capable of loading all SMFs, importing all DAW projects.
    - Exporting to all music data formats and all DAW projects.
  - Full settings compatibility. Capable of loading and operating with all past settings.
- Robustness
  - Absolutely no bugs, no matter what you do. The app never crashes.
- Usability
  - Extreme pursuit of convenience and ease of use.
- Performance
  - High-speed operation with no CPU load in any environment. Sound never cuts out.
- User Requests
  - Immediate response to all user requests.

## For Developers: Build Instructions
- Clone the necessary external repositories and build with Cargo.
- Details will be written later.

## License
- MIT