import { AbsoluteFill, Composition, registerRoot } from 'remotion';
import demo from '../../../clips/demo_basics.json';
import { HandkitClip, clipFrames } from './HandkitClip.jsx';

const FPS = demo.fps ?? 30;
const [WIDTH, HEIGHT] = demo.size ?? [1920, 1080];

/** The demo: clips/demo_basics.json on a plain backdrop, in the clip's look or `look`. */
export const Demo = ({ look, background }) => (
  <AbsoluteFill style={{ background }}>
    <HandkitClip clip={demo} look={look} />
  </AbsoluteFill>
);

const Root = () => (
  <Composition
    id="HandkitDemo"
    component={Demo}
    durationInFrames={clipFrames(demo, FPS)}
    fps={FPS}
    width={WIDTH}
    height={HEIGHT}
    defaultProps={{ look: null, background: '#2b2e33' }}
  />
);

registerRoot(Root);
