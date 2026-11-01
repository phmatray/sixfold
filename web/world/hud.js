// The world's screen furniture, in plain DOM: the D-pad and RUN (in `controls`, after the header), the warp
// fade, the grid legend, the area's banner and a scene's caption. world.js wires the input to them.
import { tr } from "./map.js";

const ARROW = '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M12 5l8 11H4z" fill="currentColor"/></svg>';

/** Builds it into `el` and `controls`. `subjectY()` says where a caption's subject is on screen
 * ({ mid, top, foot }, px from the top of `el`), or null. */
export function createHud(el, controls, subjectY) {
  const pad = document.createElement("div");
  pad.className = "world-pad";
  pad.setAttribute("role", "group");
  pad.setAttribute("aria-label", tr("Walk", "Marcher"));
  for (const [f, label] of [["n", tr("Up", "Haut")], ["w", tr("Left", "Gauche")], ["e", tr("Right", "Droite")], ["s", tr("Down", "Bas")]]) {
    const b = document.createElement("button");
    b.type = "button";
    b.innerHTML = ARROW;
    b.className = `pad-${f}`;
    b.setAttribute("aria-label", label);
    b.dataset.dir = f;
    pad.appendChild(b);
  }
  let running = false; // RUN toggled on (a phone has no Shift)
  const run = document.createElement("button");
  run.type = "button";
  run.className = "world-run";
  run.textContent = tr("Run", "Courir");
  run.hidden = true;
  run.setAttribute("aria-pressed", "false");
  run.addEventListener("click", () => {
    running = !running;
    run.setAttribute("aria-pressed", String(running));
  });
  controls.append(pad, run);
  const fade = document.createElement("div");
  fade.className = "world-fade";
  const legend = document.createElement("div");
  legend.className = "world-legend";
  legend.hidden = true;
  legend.innerHTML = tr('<span class="k-block">Blocked</span><span class="k-act">Look or use</span><span class="k-warp">Door</span><span class="k-npc">Person</span>',
    '<span class="k-block">Bloqué</span><span class="k-act">Regarder ou utiliser</span><span class="k-warp">Porte</span><span class="k-npc">Personne</span>');
  // The area's name, shown for a moment on the way in; also a gate that just opened.
  const banner = document.createElement("div");
  banner.setAttribute("aria-hidden", "true");
  Object.assign(banner.style, {
    position: "absolute", zIndex: 2, left: "50%", top: "22%", transform: "translateX(-50%)", pointerEvents: "none",
    padding: "8px 18px", borderRadius: "999px", background: "rgba(13, 26, 44, 0.72)", color: "#f6ecd2",
    font: "700 clamp(16px, 2.4vw, 22px) 'Bricolage Grotesque', system-ui, sans-serif", letterSpacing: "0.02em",
    width: "max-content", maxWidth: "calc(100% - 32px)", textAlign: "center", opacity: 0, transition: "opacity 0.5s ease",
  });
  let bannerTimer = 0;
  const showBanner = (text, ms = 2200) => {
    banner.textContent = text;
    banner.style.opacity = 1;
    clearTimeout(bannerTimer);
    bannerTimer = setTimeout(() => (banner.style.opacity = 0), ms);
  };
  // A scene's caption (a reveal's line, the door's bells, #155): a paper plate, never mistaken for the area's
  // name on its dark pill.
  // A talk box hides it (style.css `.scene-caption`).
  const caption = document.createElement("div");
  caption.className = "scene-caption";
  caption.setAttribute("aria-hidden", "true");
  Object.assign(caption.style, {
    position: "absolute", zIndex: 2, left: "50%", bottom: "18%", transform: "translateX(-50%)", pointerEvents: "none",
    padding: "10px 20px", borderRadius: "12px", background: "var(--paper)", color: "var(--toon)", border: "2.5px solid var(--toon)",
    boxShadow: "0 3px 0 var(--toon)", font: "600 clamp(15px, 2vw, 19px) 'Bricolage Grotesque', system-ui, sans-serif",
    width: "max-content", maxWidth: "calc(100% - 32px)", textAlign: "center", textWrap: "balance", opacity: 0, transition: "opacity 0.4s ease",
  });
  let captionTimer = 0;
  // Placed from what it names on screen (the seal stone, or the door's medallion): beside it, on the side
  // with more room, never over it (#155).
  const placeCaption = (fresh = false) => {
    const at = subjectY(), H = el.clientHeight, gap = 28;
    if (at && fresh) caption.above = at.mid > H / 2; // the side is picked once per caption: it never flips mid-move
    if (!at) Object.assign(caption.style, { top: "auto", bottom: "18%" });
    else if (caption.above) Object.assign(caption.style, { top: "auto", bottom: `${Math.round(Math.min(H * 0.85, Math.max(16, H - at.top + gap)))}px` });
    else Object.assign(caption.style, { bottom: "auto", top: `${Math.round(Math.min(H * 0.85, at.foot + gap))}px` });
  };
  const showCaption = (text, ms) => {
    placeCaption(true);
    caption.textContent = text;
    caption.style.opacity = 1;
    clearTimeout(captionTimer);
    captionTimer = setTimeout(() => (caption.style.opacity = 0), ms);
  };
  // A scene's end takes its caption with it (a skip, a cut back under reduced motion).
  const clearCaption = () => (clearTimeout(captionTimer), (caption.style.opacity = 0));
  el.append(fade, legend, banner, caption);

  return {
    pad, run, fade, legend, showBanner, showCaption, clearCaption,
    /** RUN is on. */
    get running() {
      return running;
    },
    /** Every frame: a caption on screen is placed again, as the camera is often still moving onto what it names (#155). */
    follow: () => caption.style.opacity === "1" && placeCaption(),
    remove() {
      clearTimeout(bannerTimer);
      clearTimeout(captionTimer);
      for (const n of [pad, run, fade, legend, banner, caption]) n.remove();
    },
  };
}
