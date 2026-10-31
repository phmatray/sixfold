// The world's look (#225): the frame drawn through a post-processing composer, in the
// renderer's own context (never a second one). `createLook(renderer, scene, camera)`:
//   high  the scene into a half-float buffer, ambient occlusion (N8AO, half resolution),
//         then one pass for bloom, the grade (saturation, contrast) and the tone mapping
//   low   the scene, then one pass for the grade and the tone mapping
//   off   today's plain frame: clear, draw
// It starts on "off" and moves to the tier asked for once postprocessing and N8AO have
// loaded (mount stays synchronous); `tier` is the one in force. The renderer keeps
// NoToneMapping: the tone mapping effect does it, and reads renderer.toneMappingExposure.
import * as THREE from "three";

export function createLook(renderer, scene, camera) {
  let lib = null, want = "off", tier = "off", composer = null, fx = null, warned = false;
  const grade = { exposure: 1, saturation: 0, contrast: 0, bloom: 1 };

  const apply = () => {
    renderer.toneMappingExposure = grade.exposure;
    if (!fx) return;
    fx.hueSat.saturation = grade.saturation;
    fx.contrast.contrast = grade.contrast;
    if (fx.bloom) fx.bloom.intensity = 0.6 * grade.bloom;
  };

  const drop = () => {
    composer?.dispose(); // its buffers, passes and their effects
    composer = fx = null;
  };

  const build = () => {
    drop();
    tier = lib ? want : "off";
    if (tier === "off") return;
    const { EffectComposer, RenderPass, EffectPass, BloomEffect, HueSaturationEffect, BrightnessContrastEffect, ToneMappingEffect, ToneMappingMode, N8AOPostPass } = lib;
    const high = tier === "high";
    composer = new EffectComposer(renderer, {
      frameBufferType: high ? THREE.HalfFloatType : THREE.UnsignedByteType,
      multisampling: Math.min(4, renderer.capabilities.maxSamples), // the canvas's antialias no longer reaches the scene
    });
    composer.addPass(new RenderPass(scene, camera)); // clears colour and depth first (the renderer has autoClear off)
    fx = { hueSat: new HueSaturationEffect(), contrast: new BrightnessContrastEffect() };
    const tone = new ToneMappingEffect({ mode: ToneMappingMode.NEUTRAL });
    if (high) {
      const size = renderer.getDrawingBufferSize(new THREE.Vector2());
      const ao = new N8AOPostPass(scene, camera, size.x, size.y);
      ao.setQualityMode("Performance");
      // Subtle: contact shade in corners and under props (a tile is 1 unit), not a halo.
      Object.assign(ao.configuration, { halfRes: true, aoRadius: 1.5, distanceFalloff: 1, intensity: 2.5, transparencyAware: false });
      composer.addPass(ao);
      fx.bloom = new BloomEffect({ luminanceThreshold: 1, mipmapBlur: true, intensity: 0.6 });
      composer.addPass(new EffectPass(camera, fx.bloom, fx.hueSat, fx.contrast, tone));
    } else composer.addPass(new EffectPass(camera, fx.hueSat, fx.contrast, tone));
    apply();
  };

  renderer.toneMapping = THREE.NoToneMapping;
  Promise.all([import("postprocessing"), import("n8ao")]).then(
    ([pp, n8]) => {
      if (lib === false) return; // disposed meanwhile
      lib = { ...pp, N8AOPostPass: n8.N8AOPostPass };
      build();
    },
    (e) => warned || ((warned = true), console.warn("Graphics: post-processing unavailable, drawing plain", e)),
  );

  return {
    /** The tier in force ("off" until the composer has loaded). */
    get tier() {
      return tier;
    },
    /** "high" | "low" | "off": the composer rebuilt in the same context. */
    setTier(t) {
      if (t === want) return;
      want = t;
      if (lib) build();
    },
    /** The day's grade (daylight.js): uniforms only. */
    setGrade(g) {
      Object.assign(grade, g);
      apply();
    },
    setSize(w, h) {
      composer?.setSize(w, h);
    },
    render(dt) {
      if (composer) return composer.render(dt);
      renderer.clear();
      renderer.render(scene, camera);
    },
    dispose() {
      drop();
      lib = false;
    },
  };
}
