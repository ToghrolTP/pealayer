/** Hit-test the object-fit: contain picture, not its letterbox container. */
export function insideMediaPicture(x: number, y: number,
  rect: {left: number; top: number; width: number; height: number}, width: number, height: number): boolean {
  if (![x,y,rect.left,rect.top,rect.width,rect.height,width,height].every(Number.isFinite)
    || width <= 0 || height <= 0 || rect.width <= 0 || rect.height <= 0) return false;
  const scale = Math.min(rect.width / width, rect.height / height);
  const left = rect.left + (rect.width - width * scale) / 2;
  const top = rect.top + (rect.height - height * scale) / 2;
  return x >= left && x <= left + width * scale && y >= top && y <= top + height * scale;
}
