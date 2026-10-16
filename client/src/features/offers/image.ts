/** Longest side of uploaded offer images: sharp on phones, a few hundred KB. */
const MAX_SIDE = 1600
/** The server accepts up to 900 KB. */
const MAX_BYTES = 900 * 1024

/**
 * Shrinks a picked image to at most 1600 px on its longest side and re-encodes it as JPEG,
 * stepping the quality down until it fits the upload limit.
 */
export async function prepareImage(file: File): Promise<Blob> {
  if (!/^image\/(jpeg|png|webp)$/.test(file.type)) {
    throw new Error('Use a JPEG, PNG or WebP image.')
  }
  const bitmap = await createImageBitmap(file)
  const scale = Math.min(1, MAX_SIDE / Math.max(bitmap.width, bitmap.height))
  const canvas = document.createElement('canvas')
  canvas.width = Math.round(bitmap.width * scale)
  canvas.height = Math.round(bitmap.height * scale)
  const ctx = canvas.getContext('2d')
  if (!ctx) throw new Error('Could not read the image.')
  // JPEG has no transparency: put transparent PNGs on white.
  ctx.fillStyle = '#ffffff'
  ctx.fillRect(0, 0, canvas.width, canvas.height)
  ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height)
  bitmap.close()

  for (const quality of [0.85, 0.75, 0.6, 0.45]) {
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/jpeg', quality))
    if (blob && blob.size <= MAX_BYTES) return blob
  }
  throw new Error('This image is too large even after resizing. Try a smaller one.')
}
