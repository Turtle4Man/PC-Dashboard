// Icons are downscaled before being stored in localStorage: the source file
// can be a photo of several megabytes, while a button only needs 128 px.
const ICON_PIXEL_SIZE = 128;

export async function fileToIcon(file) {
  if (!file.type.startsWith("image/")) {
    throw new Error("Le fichier sélectionné n'est pas une image.");
  }

  const bitmap = await createImageBitmap(file);
  try {
    const canvas = document.createElement("canvas");
    canvas.width = ICON_PIXEL_SIZE;
    canvas.height = ICON_PIXEL_SIZE;

    const context = canvas.getContext("2d");
    if (!context) {
      throw new Error("Impossible de préparer l'image.");
    }

    const scale = Math.min(
      ICON_PIXEL_SIZE / bitmap.width,
      ICON_PIXEL_SIZE / bitmap.height,
    );
    const width = Math.max(1, Math.round(bitmap.width * scale));
    const height = Math.max(1, Math.round(bitmap.height * scale));

    context.drawImage(
      bitmap,
      Math.round((ICON_PIXEL_SIZE - width) / 2),
      Math.round((ICON_PIXEL_SIZE - height) / 2),
      width,
      height,
    );

    return canvas.toDataURL("image/png");
  } finally {
    bitmap.close?.();
  }
}
