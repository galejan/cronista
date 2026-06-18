/**
 * Creates a debounced version of a function that delays invocation until
 * `ms` milliseconds have elapsed since the last call to `trigger`.
 *
 * On each call to `trigger`, any pending invocation is cancelled and the
 * timer resets.  Call `cancel` to clear the timer without invoking.
 *
 * @param fn - The function to debounce.
 * @param ms - Delay in milliseconds.
 * @returns An object with `trigger` (the debounced caller) and `cancel`
 *          (clears any pending invocation).
 */
export function debounce<Args extends unknown[]>(
  fn: (...args: Args) => void,
  ms: number,
): { trigger: (...args: Args) => void; cancel: () => void } {
  let timer: ReturnType<typeof setTimeout> | null = null;

  function cancel(): void {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  }

  function trigger(...args: Args): void {
    cancel();
    timer = setTimeout(() => {
      timer = null;
      fn(...args);
    }, ms);
  }

  return { trigger, cancel };
}
