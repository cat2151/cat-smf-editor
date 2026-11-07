# cat-smf-editor

A lightweight application aiming to easily edit SMF (Standard MIDI Files). It utilizes [UAPMD](https://github.com/atsushieno/uapmd) and is written in Rust.

Its editing features are idiosyncratic; it's primarily for personal use. Specifications may change tomorrow, potentially making previous files unreadable.

## Download

[Download Windows version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

Since this is an experimental project, it might occasionally return a 404 error.

## Past Challenges and What This Application Solves
### SMF Editing
- Attempts to publish CC0 licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- Serious performance and editing often require preliminary steps, such as launching a DAW.
- Lightweight applications for casual playing and editing sometimes had a user experience that was "not for me".
- In modern times, if you have a desired UX, you can achieve it through "vibe coding".
- So I decided to do just that.
### Enjoying Casual, Simple Composition-like Play
- Existing SMF editors and DAWs often didn't align with my desired workflow.
  - For casual, simple composition-like play, they were often paid, required account registration, had high hurdles to get audio plugins working, or high hurdles to make the first sound, etc.
- In modern times, if you have a desired UX, you can achieve it through "vibe coding".
- So I decided to do just that.

## Concepts, What it Aims For
### A Toy
- It aims to be more of a toy than an editor.
- The positioning of this application is roughly [Toy] >> Educational >> Commercial.
- Only a very limited number of elements can be edited.
- It has "editor" in its name, but don't mind that.
### Making Sound
- It aims to maintain "making sound in the author's environment" as much as possible.
### Immediate Sound
- From application launch to sound playback for a single track, it feels like 0.2 seconds. It prioritizes the fastest possible playback.
  - Processes like GUI display preparation are also handled *after* the sound.
  - Sound playing before the GUI appears might be described as a "sound splash screen".
### Implementing Only My Own Workflow
- Due to nearly zero development resources, this is a triage situation.
- Developing this type of editor requires enormous development costs.
  - For example, just to achieve both safe editing and a standard UX with features like intelligent undo/redo, time travel, and non-destructive editing, the development cost is immense. This includes directing data model design, operational design, requirement elicitation, verification implementations, and dogfooding, as well as selecting/discarding features (which, if aiming for standard, means it has to be all-encompassing).
- If you are looking for a common "lightweight and standard editor," please look elsewhere.
### From Educational to Experimental
- It leans more towards educational use than commercial.
- However, due to the aforementioned constraints, this application
- leans more towards being a toy or experimental rather than educational.
### Experimentation, Exploration, Personal Use
- Destructive changes will be made frequently.
  - It will be commonplace for files saved yesterday not to load today.
  - Therefore, please keep its use to a toy level.

## Casual Thoughts Corner
- SMF editors have various uses and too many considerations; aiming for a "practical level" incurs astronomical development costs. For example, a list might include:
  - A toy. Want to edit only specific parts thoroughly. Tweak timbres, effects, tempo curves, arpeggios, and individual notes on a piano roll. Prioritize a rapid edit-and-verify cycle, with fully automated load/save without manual intervention. → The workflow targeted by this application is a small subset of this.
  - Quickly browse and play, quickly visualize content. A lightweight browser or viewer.
  - Make minor changes. Ensure that everything except the changed part remains completely original, with diff visualization for this purpose. A lightweight editor.
  - A toy for composition and arrangement practice.
    - Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but development costs would be enormous.
    - No, I think it's better to compose with other standard modern applications, not an SMF editor.
  - Serious composition and arrangement.
    - Same as above.
    - Furthermore, achieving advanced undo, time travel, and non-destructive editing at an extremely high level is essential.
- To be added later.

## Out of Scope, What It Does Not Aim For
- DAW
  - Directly read, write, and edit not only SMF but also DAW projects from other popular DAWs, including Audio.
- Plugins
  - Freely extend all editor functionalities with plugins.
- Scripting
  - Freely extend all editor functionalities with scripts.
- Standard
  - 200% coverage of the look and behavior of standard SMF editors and DAWs. Fully backward compatible with all editors and DAWs.
- Features
  - Equipped with all possible features. Usable for all purposes.
- Compatibility
  - Full data compatibility
    - Able to load all SMFs, import all DAW projects.
    - Export compatible with all music data formats and all DAW projects.
  - Full settings compatibility. Able to load and operate with all past settings.
- Robustness
  - Absolutely no bugs, no matter what you do. The application never crashes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - High-speed operation with no CPU load in any environment. Sound never breaks up.
- User Requests
  - Immediately responding to all user requests.

## For Developers: Build Steps
- Clone the necessary external repositories and build with cargo.
- More details will be added later.

## License
- MIT