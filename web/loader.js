// Trunk initializer (data-initializer in index.html): drives the static
// loading screen (#loader) while the game downloads, then removes it; on a
// failure, or a download that stalls, it says so and offers Retry (a reload:
// the worker may have the files by then, and nothing has to be reinstalled).
export default function loader() {
  const el = document.getElementById("loader");
  const bar = el?.querySelector(".progress-bar span");
  const msg = el?.querySelector(".loader-msg");
  // The app sets <html lang> later: read the same choice it does.
  let fr = false;
  try { fr = (localStorage.getItem("sixfold.lang") ?? navigator.language).toLowerCase().startsWith("fr"); } catch {}
  const t = (en, f) => (fr ? f : en);
  // A download that breaks mid-way never settles (Trunk's reader has no
  // catch), so no news for STALL ms, or the browser going offline, fails it.
  const STALL = 20000;
  let stall = 0;
  const watch = () => {
    clearTimeout(stall);
    stall = setTimeout(() => fail("stalled"), STALL);
  };
  const offline = () => fail("offline");
  const fail = (error) => {
    clearTimeout(stall);
    removeEventListener("offline", offline);
    console.error("sixfold: the game could not load", error);
    if (!el || el.classList.contains("failed")) return;
    el.classList.add("failed");
    el.setAttribute("role", "alert");
    msg.textContent = t(
      "The game could not load. Check your connection, then try again.",
      "Le jeu n'a pas pu se charger. Vérifiez votre connexion, puis réessayez.",
    );
    const retry = el.querySelector("button");
    retry.textContent = t("Retry", "Réessayer");
    retry.hidden = false;
    retry.onclick = () => location.reload();
    retry.focus();
  };
  return {
    onStart: () => {
      if (msg) msg.textContent = t("Loading the game…", "Chargement du jeu…");
      el?.querySelector(".progress-bar")?.setAttribute("aria-label", t("Loading the game", "Chargement du jeu"));
      addEventListener("offline", offline);
      watch();
    },
    onProgress: ({ current, total }) => {
      // Downloaded: what is left (compiling the game) can't stall on the network.
      if (total && current >= total) {
        clearTimeout(stall);
        removeEventListener("offline", offline);
      } else {
        watch();
      }
      if (!bar || !total) return;
      const pct = Math.min(100, Math.floor((current * 100) / total));
      bar.style.width = `${pct}%`;
      el.querySelector(".progress-bar").setAttribute("aria-valuenow", pct);
    },
    onSuccess: () => {
      clearTimeout(stall);
      removeEventListener("offline", offline);
      el?.remove();
    },
    onFailure: fail,
  };
}
