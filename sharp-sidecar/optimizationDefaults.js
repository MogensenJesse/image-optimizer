const optimizationDefaults = {
  jpeg: {
    quality: 90,
    mozjpeg: true,
    chromaSubsampling: '4:4:4',
    optimiseCoding: true
  },
  png: {
    compressionLevel: 9,
    adaptiveFiltering: true,
    palette: true,
    quality: 90,
    effort: 10,
  },
  webp: {
    quality: 90,
    effort: 6,
    lossless: false,
    smartSubsample: true
  },
  avif: {
    quality: 90,
    effort: 8,
    chromaSubsampling: '4:4:4'
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