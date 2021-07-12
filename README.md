# corrupted_squares
Minimal Example of vulkano rendering glitches

## Behaviour

On some machines the textures get corrupted with other textures in the array. May be related to [#1588](https://github.com/vulkano-rs/vulkano/pull/1588)

### Linux with AMD Radeon RX 570:

Data from the previous texture is seen corrupting the next square. This trend continues as you add squares and textures.

![Linux AMD Screenshot](/screenshots/linux_amd-radeon-rx-570.png)

### Windows with AMD RX 5700 XT:

Same behaviour as above, except whenever I try to take a screenshot the process stops responding and crashes.

### Windows with NVIDIA GTX 1050-TI Max-Q

Renders as expected, though the colors are off but that is most likely due to lack of gamma correction.

![Windows NVIDIA Screenshot](/screenshots/windows_nvidia-gtx-1050ti-maxq.png)

### Linux with Intel Iris Plus G4:

Textures are completely corrupted, also for some reason winit isn't fullscreen. Probably related to [#1551](https://github.com/vulkano-rs/vulkano/issues/1551)

![Linux Intel Screnshot](/screenshots/linux_intel-iris-plus-g4.png)
