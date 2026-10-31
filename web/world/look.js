// The world's look (#225), in the renderer's own context (never a second one).
// `createLook(renderer, scene, camera)`:
//   high  through a post-processing composer: the scene into a half-float multisampled buffer,
//         ambient occlusion (N8AO, half resolution), then one pass for bloom, the grade
//         (saturation, contrast) and the tone mapping; the renderer keeps NoToneMapping
//   low   the plain draw with the renderer's own neutral tone mapping and the grade's exposure
//         (no saturation or contrast): no composer, no libraries, cheap
//   off   today's plain frame: clear, draw
// High fetches postprocessing and N8AO when first asked for (mount stays synchronous) and is "off" until they are in;
// `tier` is the one in force, `ready` settles once the tier asked for is (or can't be).
import * as THREE from "three";

export function createLook(renderer, scene, camera) {
  let lib = null, want = "off", tier = "off", composer = null, fx = null, warned = false;
  const grade = { exposure: 1, saturation: 0, contrast: 0, bloom: 1 };
  const warn = (msg, e) => warned || ((warned = true), console.warn(msg, e));

  const apply = () => {
    renderer.toneMappingExposure = grade.exposure;
    if (!fx) return;
    fx.hueSat.saturation = grade.saturation;
    fx.contrast.contrast = grade.contrast;
    fx.bloom.intensity = 0.6 * grade.bloom;
  };

  const drop = () => {
    composer?.dispose(); // its buffers, passes and their effects
    composer = fx = null;
  };

  const build = () => {
    drop();
    tier = want === "high" && !lib ? (load(), "off") : want;
    renderer.toneMapping = tier === "low" ? THREE.NeutralToneMapping : THREE.NoToneMapping; // three recompiles on a change
    if (tier !== "high") return apply();
    let ao = null;
    try {
      const { EffectComposer, RenderPass, EffectPass, BloomEffect, HueSaturationEffect, BrightnessContrastEffect, ToneMappingEffect, ToneMappingMode, N8AOPostPass } = lib;
      composer = new EffectComposer(renderer, {
        frameBufferType: THREE.HalfFloatType,
        multisampling: Math.min(4, renderer.capabilities.maxSamples), // the canvas's antialias no longer reaches the scene
      });
      composer.addPass(new RenderPass(scene, camera)); // clears colour and depth first (the renderer has autoClear off)
      const size = renderer.getDrawingBufferSize(new THREE.Vector2());
      ao = new N8AOPostPass(scene, camera, size.x, size.y);
      ao.setQualityMode("Performance");
      // Subtle: contact shade in corners and under props (a tile is 1 unit), not a halo.
      Object.assign(ao.configuration, { halfRes: true, aoRadius: 1.5, distanceFalloff: 1, intensity: 2.5, transparencyAware: false });
      composer.addPass(ao);
      ao = null; // the composer's now
      fx = { hueSat: new HueSaturationEffect(), contrast: new BrightnessContrastEffect(), bloom: new BloomEffect({ luminanceThreshold: 1, mipmapBlur: true, intensity: 0.6 }) };
      composer.addPass(new EffectPass(camera, fx.bloom, fx.hueSat, fx.contrast, new ToneMappingEffect({ mode: ToneMappingMode.NEUTRAL })));
    } catch (e) {
      ao?.dispose?.();
      drop();
      warn("Graphics: High could not be built, drawing Low", e);
      return (want = "low"), build();
    }
    apply();
  };

  // Fetched the first time High is asked for: Low and Off never parse the two libraries.
  let loading = null;
  const load = () =>
    (loading ??= Promise.all([import("postprocessing"), import("n8ao")]).then(
      ([pp, n8]) => {
        if (lib === false) return; // disposed meanwhile
        lib = { ...pp, N8AOPostPass: n8.N8AOPostPass };
        if (want === "high") build();
      },
      (e) => warn("Graphics: post-processing unavailable, drawing plain", e),
    ));
  build();

  return {
    /** The tier in force (High is "off" until the composer has loaded). */
    get tier() {
      return tier;
    },
    /** Settles once the tier asked for is in force, or can't be (the libraries failed). */
    get ready() {
      return want === "high" && lib === null ? load() : Promise.resolve();
    },
    /** "high" | "low" | "off": rebuilt in the same context. */
    setTier(t) {
      if (t === want) return;
      want = t;
      build();
    },
    /** The day's grade (daylight.js): uniforms only. */
    setGrade(g) {
      Object.assign(grade, g);
      apply();
    },
    /** Each frame, after `w.day.update`: lamps shine past the bloom threshold only on High (a change
     * rewrites the glow materials on the next update; fireflies read `w.glowBoost`), and the hour's grade. */
    follow(w, lit) {
      const boost = tier === "high";
      if (w.glowBoost !== boost) (w.glowBoost = boost), (w.lightDirty = true);
      grade.exposure = lit.exposure;
      grade.saturation = lit.saturation;
      grade.contrast = lit.contrast;
      grade.bloom = lit.bloom;
      apply();
    },
    setSize(w, h) {
      composer?.setSize(w, h);
    },
    /** Compiles the scene's programs for the target they will draw into (High: the composer's buffer). */
    warm() {
      if (!composer) return renderer.compileAsync(scene, camera);
      const was = renderer.getRenderTarget();
      renderer.setRenderTarget(composer.inputBuffer);
      const p = renderer.compileAsync(scene, camera); // compile() runs synchronously at its start
      renderer.setRenderTarget(was);
      return p;
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
