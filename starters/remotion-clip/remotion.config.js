import { Config } from '@remotion/cli/config';

// WebGL2 on the GPU (ANGLE); use 'swiftshader' on a machine without one.
Config.setChromiumOpenGlRenderer('angle');
Config.setVideoImageFormat('png');

// @handkit/three is linked from packages/ (`file:`): keep its node_modules path so its
// `three` import finds this project's copy.
Config.overrideWebpackConfig((config) => ({
  ...config,
  resolve: { ...config.resolve, symlinks: false },
}));
