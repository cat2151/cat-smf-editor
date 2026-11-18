# cat-smf-editor

A lightweight application aiming to easily edit SMF (Standard MIDI File). It utilizes [UAPMD](https://github.com/atsushieno/uapmd). Written in Rust.

Its editing features have quirks. It's almost exclusively for personal use. Specifications may change tomorrow, potentially rendering existing files unreadable.

## Download

[Download Windows version](https://github.com/cat2151/cat-smf-editor/releases/download/v0.1.0/cat-smf-editor-0.1.0-windows-x64.zip)

Since this is an experimental project, you might occasionally encounter a 404 error.

## Past Challenges and What This Application Solves
### SMF Editing
- Attempts to publish CC0 licensed music data, like [The Mutopia Project](https://www.mutopiaproject.org/), are wonderful!
- For full-scale performances and their editing, often extensive preparation, such as launching a DAW, is required.
- Lightweight apps for casual performance and editing often didn't align with my preferred workflow/user experience (UX).
- Nowadays, if you have a desired UX, you can achieve it through 'vibe coding'.
- So I decided to do just that.
### Enjoying Casual, Composition-like Play
- Existing SMF editors and DAWs often didn't align with my desired workflow.
  - When engaging in casual, composition-like play, barriers included: being paid, requiring account registration, high hurdles to play with audio plugins, and high hurdles to get the first sound.
- Nowadays, if you have a desired UX, you can achieve it through 'vibe coding'.
- So I decided to do just that.
## Zundamon
- I wanted to easily enjoy Zundamon's singing. Existing applications often didn't align with my desired workflow.
- Nowadays, if you have a desired UX, you can achieve it through 'vibe coding'.
- So I decided to do just that.

## Concept, What It Aims For
- Toy
  - It aims to be more of a toy than an editor.
  - The positioning of this application is roughly [Toy] >> Educational >> Commercial.
  - Only a very limited number of elements can be edited.
  - It has 'editor' in its name, but don't mind that.
- Sound Playback
  - It aims to maintain "sound playback in the author's environment" as much as possible.
- Immediate Sound Playback
  - From application launch to sound playback, it feels like 0.2 seconds for a single track. Playback is optimized for maximum speed.
    - Processes like GUI display preparation are also placed after sound playback.
      - Sound playing before the GUI appears might be considered a sound-based splash screen.
- Implementing Only My Own Workflow
  - Due to virtually zero development resources, it's a triage.
  - Developing this kind of editor requires enormous development costs.
    - For example, achieving both safe editing and a standard UX with intelligent undo/redo, time machine functionality, non-destructive editing, etc., would involve immense development costs just for directing data model design, operational design, requirement elicitation, verification implementations, dogfooding, and feature selection (which, if aiming for a standard, would mean including everything).
  - If you're looking for a common "lightweight and standard editor," please look elsewhere.
- From Educational to Experimental
  - It leans more towards educational use than commercial use.
  - However, due to the aforementioned constraints, this application
  - leans more towards a toy or experimental tool than an educational one.
- Experimentation, Exploration, Personal Use
  - Destructive changes will occur frequently.
    - It will be commonplace for files saved yesterday to be unreadable today.
      - Therefore, please limit its use to a 'toy' level.

## Casual Thoughts and Musings Corner
- Sound Dropouts
  - Multithreaded rendering has been adopted. Sound dropouts have been reduced.
  - Implementing a Track Freeze feature is under consideration.
- SMF editors have various uses and too many considerations; aiming for a "practical level" results in prohibitively high development costs. For example, listing them:
  - Toy. Want to heavily edit only specific parts. Adjusting timbres, effects, tempo curves, arpeggios, and individual notes with a piano roll. Prioritizing a quick edit and verification cycle, with fully automated load/save operations without manual intervention → This app's target workflow is only a small part of that.
  - Want to quickly browse and play, quickly visualize content. A lightweight browser or viewer.
  - Want to make minor changes. Guarantee that everything except the changed part remains perfectly original, with diff visualization for this purpose. A lightweight editor.
  - Practice toy for composition and arrangement.
    - Casual visualization, casual editing. Non-destructive editing would be good to prevent confusion, but development costs would be enormous.
    - No, I think it's better to compose with other standard modern applications, not SMF editors.
  - Serious composition and arrangement.
    - Same as above.
    - Furthermore, highly advanced undo, time machine, and non-destructive editing must be implemented at an extremely high level.
- More to be added later.

## Out of Scope, What It Does Not Aim For
- Standard or masterpiece as a beginner's DAW or educational DAW.
  - Something that even cats or babies can perfectly master all functions immediately, and also serves as preparation for using more advanced DAWs.
- DAW
  - Directly reading, writing, and editing not only SMF but also DAW projects from other popular DAWs, including Audio, of course.
- Plugins, Add-ons, Add-ins
  - Freely extending all editor functions with external DLLs.
- Scripting
  - Freely extending all editor functions with scripts.
- Standard
  - 200% coverage of the look and behavior of standard SMF editors and DAWs. Completely superseding all existing editors and DAWs.
- Features
  - Equipped with every imaginable feature. Usable for all purposes.
- Compatibility
  - Complete data compatibility.
    - Capable of loading all SMF files, importing all DAW projects.
    - Export supports all music data formats and all DAW projects.
  - Complete settings compatibility. Capable of loading and operating with all past settings.
- Robustness
  - Absolutely no bugs regardless of input. The application never crashes.
- Convenience
  - Pursuing convenience and ease of use to the extreme.
- Performance
  - High-speed operation with no CPU load in any environment. Sound never drops out.
- Feature Requests
  - Immediately responding to all user requests.

## Build Instructions for Developers
- Clone the necessary external repositories and build with Cargo.
- More details will be added later.

## License
- MIT