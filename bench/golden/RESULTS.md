# Golden renders: fframes vs three.js

44 library gestures, each posed with the skinned mesh and corrective shapes and rendered at 384 px by both engines (`python tools/golden/compare.py crates/handkit-core/gestures/*.json`). The gate is a silhouette IoU of at least 0.97.

All 44 pass: worst IoU 0.9927, mean 0.9958. Color differs by 12 to 36 of 255 on average because the two engines light differently; matching looks is M5 work.

| Gesture | Silhouette IoU | Mean color difference (/255) |
| --- | --- | --- |
| offering_palm | 0.9927 | 22.5 |
| count_on_fingers | 0.9928 | 16.0 |
| camera_frame | 0.9931 | 12.2 |
| point_at_palm | 0.9933 | 27.9 |
| triangle | 0.9936 | 29.1 |
| high_five | 0.9940 | 26.6 |
| tulip | 0.9940 | 25.8 |
| spread_palm | 0.9944 | 34.1 |
| steeple | 0.9945 | 25.4 |
| beckon | 0.9946 | 24.0 |
| ok | 0.9946 | 28.5 |
| heart_hands | 0.9947 | 16.8 |
| cupped | 0.9948 | 25.1 |
| tripod | 0.9953 | 27.8 |
| time_out | 0.9954 | 23.0 |
| vulcan | 0.9955 | 34.4 |
| relaxed | 0.9957 | 28.3 |
| open_palm | 0.9958 | 34.4 |
| peace | 0.9958 | 30.1 |
| four | 0.9959 | 31.9 |
| thumb_to_middle | 0.9960 | 29.1 |
| bunny_ears | 0.9963 | 36.4 |
| finger_gun | 0.9963 | 26.0 |
| finger_heart | 0.9963 | 35.7 |
| stop | 0.9963 | 33.7 |
| prayer | 0.9964 | 25.6 |
| thumb_to_ring | 0.9965 | 29.7 |
| pinch_open | 0.9966 | 29.1 |
| three | 0.9966 | 30.5 |
| claw | 0.9968 | 30.3 |
| fist_bump | 0.9968 | 29.6 |
| thumbs_down | 0.9968 | 25.3 |
| rock_horns | 0.9969 | 32.8 |
| count_two_thumb | 0.9970 | 32.1 |
| shaka | 0.9970 | 33.3 |
| thumb_to_pinky | 0.9970 | 30.4 |
| crossed_fingers | 0.9971 | 33.7 |
| thumbs_up | 0.9971 | 27.5 |
| pinch | 0.9972 | 28.7 |
| point | 0.9972 | 29.0 |
| fist_thumb_in | 0.9974 | 31.3 |
| snap_ready | 0.9975 | 27.6 |
| pinky_hook | 0.9976 | 35.6 |
| fist | 0.9986 | 27.9 |
