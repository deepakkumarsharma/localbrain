const SECRET_PATTERNS = [
  /sk-[a-zA-Z0-9_-]+/g,
  /api[_-]?key\s*[:=]\s*['"]?[^'"\s]+/gi,
  /token\s*[:=]\s*['"]?[^'"\s]+/gi,
];

export function redactSensitiveText(value: string) {
  return SECRET_PATTERNS.reduce(
    (current, pattern) => current.replace(pattern, '[redacted]'),
    value,
  );
}

export function friendlyErrorMessage(value: string) {
  const redacted = redactSensitiveText(value);

  if (redacted.includes('escapes workspace root')) {
    return 'That path is outside the active workspace, so Local Brain blocked it.';
  }
  if (redacted.includes('No such file') || redacted.includes('os error 2')) {
    return 'That file could not be found. Pick an indexed source file and try again.';
  }
  if (redacted.includes('unsupported source file')) {
    return 'That file type is not supported by the parser yet.';
  }

  return redacted;
}
