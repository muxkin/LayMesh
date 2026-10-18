// Keep every original copyright and permission text used by this runtime.
// Only exact duplicates and components absent from a playback copy are removed.
export function selectWebLicenses(bytes, editable) {
  const inventory = JSON.parse(new TextDecoder().decode(bytes));
  if (inventory.schemaVersion !== 1) return bytes;
  const mode = typeof editable === "string" ? editable : editable ? "editable" : "playback";
  const packages = inventory.packages.filter((p) => p.profiles.includes(mode));
  const used = new Set(packages.flatMap((p) => p.texts.map((r) => r.sha256)));
  const texts = Object.fromEntries(
    Object.entries(inventory.texts).filter(([hash]) => used.has(hash)),
  );
  return new TextEncoder().encode(
    JSON.stringify({ ...inventory, packages, texts }),
  );
}
