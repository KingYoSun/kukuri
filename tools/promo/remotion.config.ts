import { Config } from '@remotion/cli/config';

// 媒体別出力の既定。品質に関わる値は composition 側ではなくここで一元管理する。
// 文字の多い UI 画面が素材なので、中間 frame は jpeg ではなく png にする。
// jpeg にすると full range と判定され、pixel format が yuvj420p になる。
Config.setVideoImageFormat('png');
Config.setCodec('h264');
// LP・SNS のどの再生環境でも同じ色で再生できるようにする。
Config.setPixelFormat('yuv420p');
Config.setColorSpace('bt709');
Config.setOverwriteOutput(true);
// 撮影素材の取り込み先。capture が書き出す promo-artifacts を public として読む。
Config.setPublicDir('../../promo-artifacts');
