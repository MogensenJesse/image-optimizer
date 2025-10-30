/**
 * Default optimization settings for different image formats
 * @module config/formats/defaults
 */

const jpeg = {
  quality: 90,
  mozjpeg: true,
  chromaSubsampling: "4:2:0",
  optimiseCoding: true,
};

const png = {
  compressionLevel: 7,
  adaptiveFiltering: true,
  palette: true,
  effort: 4,
};

const webp = {
  quality: 90,
  alphaQuality: 90,
  effort: 4,
  lossless: false,
  smartSubsample: false,
};

const avif = {
  quality: 90,
  effort: 2,
  chromaSubsampling: "4:2:0",
  lossless: false,
};

const tiff = {
  quality: 100,
  compression: "deflate",
  predictor: "horizontal",
  pyramid: false,
  tile: true,
  tileWidth: 256,
  tileHeight: 256,
};

module.exports = {
  jpeg,
  png,
  webp,
  avif,
  tiff,
};
