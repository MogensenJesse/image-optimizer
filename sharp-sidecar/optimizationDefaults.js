const optimizationDefaults = {
  jpeg: {
    quality: 90,
    mozjpeg: true,
    chromaSubsampling: '4:2:0',
    optimiseCoding: true
  },
  png: {
    compressionLevel: 7,
    adaptiveFiltering: true,
    palette: true,
    effort: 4,
  },
  webp: {
    quality: 90,
    alphaQuality: 90,
    effort: 4,
    lossless: false,
    smartSubsample: false
  },
  avif: {
    quality: 90,
    effort: 2,
    chromaSubsampling: '4:2:0',
    lossless: false,
  },
  tiff: {
    quality: 100,
    compression: 'deflate',
    predictor: 'horizontal',
    pyramid: false,
    tile: true,
    tileWidth: 256,
    tileHeight: 256
  }
};

module.exports = optimizationDefaults; 