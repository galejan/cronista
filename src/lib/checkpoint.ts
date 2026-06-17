/**
 * Starts an inactivity timer that would trigger a Git checkpoint after the
 * specified interval.
 *
 * This is a skeleton — the real timer logic (word-count threshold,
 * idle-detection) will be implemented in a later change.
 *
 * @param proyectoPath - The filesystem path to the project root.
 * @param intervalMs   - Checkpoint interval in milliseconds (e.g. 1_800_000
 *                       for 30 minutes).
 * @returns A cleanup function that stops the timer when called.
 */
export function startCheckpointTimer(
  proyectoPath: string,
  intervalMs: number,
): () => void {
  const intervalId = setInterval(() => {
    console.log(
      `[Checkpoint] Would fire checkpoint for project: ${proyectoPath}`,
    );
  }, intervalMs);

  return () => clearInterval(intervalId);
}
