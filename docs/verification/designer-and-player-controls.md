# Designer and player controls

The effect designer's repeat projections and palette are documented in
[Step repeats and designer ghost instances](step-repeat-ghosts.md). Their
interaction is visual authoring only and never sends hardware output commands.

The player and monitor track selectors previously used the same egui popup ID
for the primary track menu and secondary customization menu. A primary click
opened the first and immediately closed it in the second handler. Separate popup
IDs preserve both actions; native pointer tests exercise video, audio and
subtitle selectors in both control bars without connecting a controller.

RF management belongs to Hardware Monitor, not the Workspace menu. The latter
entry came from the initial RF integration; the hardware location remains
available disconnected for reviewing and preparing a draft, and saving or
transmitting continues to follow the controller's connection and capability
checks. Opening the offline manager does not issue a board request.

The status FPS widget chooses regular text when it reports screen rendering FPS
and retains emphasis for media FPS, including when Auto changes source. Native
and Web checks exercise the source choice and font behavior.
