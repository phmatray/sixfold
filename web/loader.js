// Trunk initializer (data-initializer in index.html): drives the static
// loading screen (#loader) while the game downloads, then removes it; on a
// failure it says so and offers Retry (a reload: the worker may have the
// files by then, and nothing has to be reinstalled).
export default function loader() {
  const el = document.getElementById("loader");
  const bar = el?.querySelector(".progress-bar span");
  const msg = el?.querySelector(".loader-msg");
  // The app sets <html lang> later: read the same choice it does.
  let fr = false;
  try { fr = (localStorage.getItem("sixfold.lang") ?? navigator.language).toLowerCase().startsWith("fr"); } catch {}
  const t = (en, f) => (fr ? f : en);
  return {
    onStart: () => {
      if (msg) msg.textContent = t("Loading the game…", "Chargement du jeu…");
    },
    onProgress: ({ current, total }) => {
      if (!bar || !total) return;
      const pct = Math.min(100, Math.floor((current * 100) / total));
      bar.style.width = `${pct}%`;
      el.querySelector(".progress-bar").setAttribute("aria-valuenow", pct);
    },
    onSuccess: () => el?.remove(),
    onFailure: (error) => {
      console.error("sixfold: the game could not load", error);
      if (!el) return;
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
    },
  };
}
