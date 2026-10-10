# React Bits adaptations

Tron uses application-local adaptations of [React Bits](https://reactbits.dev/)
by David Haz. These components are not exported by Tron's publishable packages.

- `BitsContent.tsx`: [AnimatedContent](https://github.com/DavidHDev/react-bits/blob/main/src/ts-default/Animations/AnimatedContent/AnimatedContent.tsx)
  and [FadeContent](https://github.com/DavidHDev/react-bits/blob/main/src/ts-default/Animations/FadeContent/FadeContent.tsx),
  adapted to Agent UI's `MotionEffects` contract. Uses GSAP's entrance tween, with
  explicit replay and scoped cleanup instead of scroll triggers. No child remounts,
  no initial hidden styles, and no persistent transform containing blocks.
- `ShinyText.tsx` / `motion.css`: [ShinyText](https://github.com/DavidHDev/react-bits/blob/main/src/ts-default/TextAnimations/ShinyText/ShinyText.tsx)'s
  gradient sweep, reduced to CSS playback. Colors inherit the current theme;
  hidden, reduced and disabled modes show normal text.

Source reviewed on 2026-10-10. The upstream MIT + Commons Clause license is
preserved in `public/licenses/react-bits.txt`, which Vite copies into web and
desktop build output. GSAP remains a separate dependency.
