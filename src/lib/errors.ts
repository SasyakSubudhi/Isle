/** Turns a rejected promise value into a sentence fit to show the user. */
export function errorMessage(error: unknown): string {
  const text = String(error instanceof Error ? error.message : error).trim();
  const message = text.replace(/^(Error|TypeError):\s*/, '');
  return message || 'Something went wrong.';
}
