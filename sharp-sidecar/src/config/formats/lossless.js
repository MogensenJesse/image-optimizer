/**
 * Lossless optimization settings for different image formats
 * @module config/formats/lossless
 */

const jpeg = {
  quality: 100,
  mozjpeg: true,
  chromaSubsampling: "4:4:4",
  optimiseCoding: true,
};

const png = {
  compressionLevel: 9,
  palette: false,
  quality: 100,
  effort: 10,
  adaptiveFiltering: true,
};

const webp = {
  lossless: true,
  quality: 100,
  effort: 6,
  nearLossless: false,
};

const avif = {
  lossless: true,
  quality: 100,
  effort: 9,
  chromaSubsampling: "4:4:4",
};

const tiff = {
  quality: 100,
  compression: "deflate",
  predictor: "horizontal",
  pyramid: false,
  tile: true,
  tileWidth: 256,
  tileHeight: 256,
  squash: false,
  preserveIccProfile: true,
};

/**
 * Get lossless settings for a specific format
 * @param {string} format - The image format (jpeg, png, webp, avif, tiff)
 * @returns {Object} Format-specific lossless settings
 */
function getLosslessSettings(format) {
  const settings = {
    jpeg,
    png,
    webp,
    avif,
    tiff,
  };

  return settings[format] || null;
}

module.exports = {
  getLosslessSettings,
  jpeg,
  png,
  webp,
  avif,
  tiff,
};
