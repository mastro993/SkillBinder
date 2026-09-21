const darkQuery = "(prefers-color-scheme: dark)";

/**
 * shadcn tokens switch on the `dark` class. SkillBinder has no theme picker, so the class
 * follows the operating system preference and keeps following it while the app runs.
 */
export function watchSystemTheme() {
  const media = window.matchMedia(darkQuery);
  const apply = () =>
    document.documentElement.classList.toggle("dark", media.matches);
  apply();
  media.addEventListener("change", apply);
}
